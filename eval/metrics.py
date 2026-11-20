"""LLM-as-a-judge metrics for a multi-step agent trajectory.

Deterministic scores live in the Rust harness. This package scores the same
three axes with DeepEval metrics whose prompts are LangChain chains:

* spec compliance
* tool selection
* error recovery

``EVAL_LIVE=1`` swaps the judge for ChatOpenAI. Otherwise a rubric model reads
the trajectory and applies the published scoring rules, which keeps CI green
without an API key.
"""

from __future__ import annotations

import json
import os
import re
from typing import Any

os.environ.setdefault("DEEPEVAL_TELEMETRY_OPT_OUT", "1")

from deepeval.metrics import BaseMetric
from deepeval.test_case import LLMTestCase, ToolCall
from langchain_core.language_models.chat_models import BaseChatModel
from langchain_core.messages import AIMessage, BaseMessage
from langchain_core.outputs import ChatGeneration, ChatResult
from langchain_core.prompts import ChatPromptTemplate

PLANNING_TOOLS = ("print_site_urls", "check_status_code")

JUDGE_SYSTEM = """You are an evaluation judge for a multi-agent coding run.
Score exactly one dimension of a multi-step trajectory.
The trajectory lists every agent step and tool call in order.
Return only a JSON object with keys "score" (number from 0 to 1) and "reason"
(one or two sentences that cite the spec id and the step or field that decided the score).
Do not wrap the JSON in markdown.
"""

DIMENSION_RUBRIC = {
    "spec_compliance": (
        "Spec compliance: 1 point if the project description contains the expected "
        "goal phrase, 1 point for each matching project-scope boolean, and 1 point "
        "for each expected route present in the API schema. Divide by the number of "
        "points. If the build failed before routes were extracted, do not penalize "
        "missing routes."
    ),
    "tool_selection": (
        "Tool selection: recall of the expected tool names, minus 0.25 for each "
        "unexpected planning tool (print_site_urls or check_status_code). Clamp to 0..1."
    ),
    "error_recovery": (
        "Error recovery across the steps: 1 if there was no cargo_build and no agent "
        "error, or if any cargo_build succeeded (including after an earlier failure). "
        "0 if every cargo_build failed, or if the run errored before a build."
    ),
}


class RubricJudge(BaseChatModel):
    """Offline stand-in for the judge. It applies DIMENSION_RUBRIC to the trajectory."""

    @property
    def _llm_type(self) -> str:
        return "trajectory-rubric-judge"

    @property
    def _identifying_params(self) -> dict[str, Any]:
        return {"mode": "rubric"}

    def _generate(
        self,
        messages: list[BaseMessage],
        stop: list[str] | None = None,
        run_manager: Any = None,
        **kwargs: Any,
    ) -> ChatResult:
        text = "\n".join(str(message.content) for message in messages)
        dimension = _dimension_from_prompt(text)
        payload = _trajectory_from_prompt(text)
        score, reason = score_dimension(dimension, payload)
        body = json.dumps({"score": score, "reason": reason})
        return ChatResult(generations=[ChatGeneration(message=AIMessage(content=body))])


class TrajectoryJudgeMetric(BaseMetric):
    """DeepEval metric that asks a LangChain chain to score one dimension."""

    def __init__(self, dimension: str, chain: Any, threshold: float = 0.99) -> None:
        self.dimension = dimension
        self.chain = chain
        self.threshold = threshold
        self.evaluation_model = "langchain"
        self.name = {
            "spec_compliance": "Spec Compliance",
            "tool_selection": "Tool Selection",
            "error_recovery": "Error Recovery",
        }[dimension]

    def measure(self, test_case: LLMTestCase, *args: Any, **kwargs: Any) -> float:
        result = self.chain.invoke(
            {"dimension": self.dimension, "rubric": DIMENSION_RUBRIC[self.dimension], "case_json": test_case.actual_output}
        )
        content = result.content if hasattr(result, "content") else str(result)
        score, reason = parse_judge_json(content)
        self.score = score
        self.reason = reason
        self.success = score >= self.threshold
        return score

    async def a_measure(self, test_case: LLMTestCase, *args: Any, **kwargs: Any) -> float:
        return self.measure(test_case)

    @property
    def __name__(self) -> str:
        return self.name


def build_chain(live: bool | None = None) -> Any:
    prompt = ChatPromptTemplate.from_messages(
        [
            ("system", JUDGE_SYSTEM),
            (
                "human",
                "Dimension: {dimension}\nRubric: {rubric}\n\nTrajectory JSON:\n{case_json}",
            ),
        ]
    )
    if live is None:
        live = os.environ.get("EVAL_LIVE", "").strip() in {"1", "true", "yes", "on"}
    model: BaseChatModel
    if live:
        from langchain_openai import ChatOpenAI

        api_key = os.environ.get("OPENAI_API_KEY") or os.environ.get("OPEN_AI_KEY")
        if not api_key:
            raise RuntimeError("EVAL_LIVE=1 requires OPENAI_API_KEY or OPEN_AI_KEY")
        model = ChatOpenAI(
            model=os.environ.get("OPENAI_MODEL", "gpt-4o-mini"),
            api_key=api_key,
            temperature=0,
        )
    else:
        model = RubricJudge()
    return prompt | model


def score_dimension(dimension: str, trajectory: dict[str, Any]) -> tuple[float, str]:
    if dimension == "spec_compliance":
        score, detail = spec_compliance(trajectory)
    elif dimension == "tool_selection":
        score, detail = tool_selection(trajectory)
    elif dimension == "error_recovery":
        score, detail = error_recovery(trajectory)
    else:
        raise ValueError(f"unknown dimension {dimension}")
    spec_id = trajectory.get("spec_id", "unknown")
    reason = f"{spec_id}: {detail}"
    return round2(score), reason


def spec_compliance(trajectory: dict[str, Any]) -> tuple[float, str]:
    factsheet = trajectory.get("factsheet") or {}
    description = str(factsheet.get("project_description") or "").lower()
    needle = str(trajectory.get("expected_goal_contains") or "").lower()
    points = 0.0
    total = 0.0
    problems: list[str] = []

    total += 1
    if needle and needle in description:
        points += 1
    elif needle:
        problems.append(f"goal missing '{needle}'")

    total += 3
    actual_scope = factsheet.get("project_scope")
    expected_scope = trajectory.get("expected_scope") or {}
    if isinstance(actual_scope, dict):
        for field in (
            "is_crud_required",
            "is_user_login_and_logout",
            "is_external_urls_required",
        ):
            if actual_scope.get(field) == expected_scope.get(field):
                points += 1
            else:
                problems.append(f"{field} was {actual_scope.get(field)}")
    else:
        problems.append("project_scope missing")

    schema = factsheet.get("api_endpoint_schema") or []
    present = {item.get("route") for item in schema if isinstance(item, dict)}
    build_passed = _build_passed(trajectory)
    for route in trajectory.get("expected_routes") or []:
        total += 1
        if route in present:
            points += 1
        elif build_passed:
            problems.append(f"route {route} missing from schema")
        else:
            points += 1

    score = 1.0 if total == 0 else points / total
    if problems:
        return score, "spec drift at " + "; ".join(problems)
    return score, "scope, goal, and routes match the spec across the run"


def tool_selection(trajectory: dict[str, Any]) -> tuple[float, str]:
    actual = [step.get("tool") for step in trajectory.get("steps") or [] if step.get("tool")]
    expected = list(trajectory.get("expected_tools") or [])
    if not expected:
        return 1.0, "no expected tools"
    actual_set = set(actual)
    hits = sum(1 for tool in expected if tool in actual_set)
    recall = hits / len(expected)
    expected_set = set(expected)
    unexpected = sorted({tool for tool in actual if tool in PLANNING_TOOLS and tool not in expected_set})
    score = max(0.0, min(1.0, recall - 0.25 * len(unexpected)))
    missing = [tool for tool in expected if tool not in actual_set]
    if unexpected:
        return score, "unexpected planning tools " + ", ".join(unexpected)
    if missing:
        return score, "missing tools " + ", ".join(missing)
    return score, "tool trace matches the expected branch, including every multi-step call"


def error_recovery(trajectory: dict[str, Any]) -> tuple[float, str]:
    builds = [step for step in trajectory.get("steps") or [] if step.get("tool") == "cargo_build"]
    if not builds:
        if trajectory.get("agent_error"):
            return 0.0, f"run failed before a build: {trajectory['agent_error']}"
        return 1.0, "no build error to recover from"
    if any(step.get("success") for step in builds):
        if any(not step.get("success") for step in builds):
            return 1.0, "cargo_build failed and a later print_fixed_code build succeeded"
        return 1.0, "cargo_build succeeded on the first attempt"
    return 0.0, "every cargo_build failed; print_fixed_code did not recover"


def trajectory_to_case(trajectory: dict[str, Any]) -> LLMTestCase:
    steps = trajectory.get("steps") or []
    context = [
        f"{step.get('agent')} {step.get('state')} {step.get('tool')} success={step.get('success')}"
        for step in steps
    ]
    tools_called = [
        ToolCall(name=str(step.get("tool")))
        for step in steps
        if step.get("tool")
    ]
    expected_tools = [ToolCall(name=name) for name in trajectory.get("expected_tools") or []]
    return LLMTestCase(
        input=str(trajectory.get("spec") or trajectory.get("spec_id") or ""),
        actual_output=json.dumps(trajectory),
        expected_output=json.dumps(
            {
                "goal_contains": trajectory.get("expected_goal_contains"),
                "scope": trajectory.get("expected_scope"),
                "routes": trajectory.get("expected_routes"),
            }
        ),
        context=context or None,
        tools_called=tools_called or None,
        expected_tools=expected_tools or None,
        name=str(trajectory.get("spec_id") or ""),
    )


def judge_trajectory(trajectory: dict[str, Any], chain: Any) -> dict[str, Any]:
    case = trajectory_to_case(trajectory)
    judged: dict[str, Any] = {}
    for dimension in ("spec_compliance", "tool_selection", "error_recovery"):
        metric = TrajectoryJudgeMetric(dimension, chain)
        metric.measure(case)
        judged[dimension] = {
            "score": metric.score,
            "reason": metric.reason,
            "success": metric.success,
        }
    return judged


def parse_judge_json(text: str) -> tuple[float, str]:
    match = re.search(r"\{.*\}", text, re.DOTALL)
    if not match:
        raise ValueError(f"judge did not return JSON: {text[:240]}")
    payload = json.loads(match.group(0))
    score = float(payload["score"])
    reason = str(payload.get("reason") or "")
    return score, reason


def _dimension_from_prompt(text: str) -> str:
    match = re.search(r"Dimension:\s*([a-z_]+)", text)
    if not match:
        raise ValueError("judge prompt is missing Dimension")
    return match.group(1)


def _trajectory_from_prompt(text: str) -> dict[str, Any]:
    marker = "Trajectory JSON:"
    index = text.rfind(marker)
    if index < 0:
        raise ValueError("judge prompt is missing Trajectory JSON")
    raw = text[index + len(marker) :].strip()
    return json.loads(raw)


def round2(value: float) -> float:
    """Half-away-from-zero to 2 decimals, matching the Rust harness."""
    scaled = value * 100.0
    if scaled >= 0:
        return int(scaled + 0.5) / 100.0
    return -int(-scaled + 0.5) / 100.0


def _build_passed(trajectory: dict[str, Any]) -> bool:
    verification = trajectory.get("verification") or {}
    build = verification.get("build") or {}
    if build.get("success") is True:
        return True
    steps = trajectory.get("steps") or []
    for step in reversed(steps):
        if step.get("tool") == "cargo_build":
            return bool(step.get("success"))
    return False
