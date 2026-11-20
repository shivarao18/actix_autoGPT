"""Score recorded trajectories with the DeepEval judge and write a report.

    python -m eval.run_eval --trajectories docs/sample_trajectories.json \\
        --out docs/sample_eval_report.md --json docs/sample_eval_report.json

Set EVAL_LIVE=1 and OPENAI_API_KEY to use ChatOpenAI instead of the offline rubric.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path

from eval.metrics import build_chain, judge_trajectory


def main() -> None:
    parser = argparse.ArgumentParser(description="Judge agent trajectories")
    parser.add_argument("--trajectories", required=True, type=Path)
    parser.add_argument("--out", type=Path, help="Markdown report path")
    parser.add_argument("--json", dest="json_out", type=Path, help="JSON report path")
    parser.add_argument("--live", action="store_true", help="Use ChatOpenAI (requires an API key)")
    args = parser.parse_args()

    trajectories = json.loads(args.trajectories.read_text())
    if not isinstance(trajectories, list):
        raise SystemExit("trajectories file must be a JSON array")

    chain = build_chain(live=True if args.live else None)
    rows = []
    for trajectory in trajectories:
        judged = judge_trajectory(trajectory, chain)
        deterministic = trajectory.get("deterministic") or {}
        rows.append(
            {
                "spec_id": trajectory.get("spec_id"),
                "model": trajectory.get("model"),
                "prompt_version": trajectory.get("prompt_version"),
                "tool_schema_version": trajectory.get("tool_schema_version"),
                "outcome": trajectory.get("outcome"),
                "failure_modes": trajectory.get("failure_modes") or [],
                "deterministic": deterministic,
                "judge": judged,
            }
        )

    report = render(rows, live=args.live)
    if args.out:
        args.out.parent.mkdir(parents=True, exist_ok=True)
        args.out.write_text(report)
        print(f"wrote {args.out}")
    else:
        print(report)
    if args.json_out:
        args.json_out.parent.mkdir(parents=True, exist_ok=True)
        args.json_out.write_text(json.dumps(rows, indent=2) + "\n")
        print(f"wrote {args.json_out}")


def render(rows: list[dict], live: bool) -> str:
    judge_name = "ChatOpenAI (EVAL_LIVE)" if live else "offline rubric model behind a LangChain chain"
    lines = [
        "# Evaluation report",
        "",
        "Deterministic checks come from the Rust harness (build, smoke, endpoints, scope, tool trace, recovery).",
        f"The judge is a DeepEval metric per axis, prompted with LangChain. This run used the {judge_name}.",
        "A judge score below 0.99 means that axis did not fully match the rubric.",
        "",
        "## Scores",
        "",
        "| Spec | Version | Outcome | Spec det / judge | Tools det / judge | Recovery det / judge |",
        "| --- | --- | --- | --- | --- | --- |",
    ]
    for row in rows:
        version = f"{row['model']} / {row['prompt_version']} / {row['tool_schema_version']}"
        det = row["deterministic"]
        judge = row["judge"]
        lines.append(
            "| {spec} | {version} | {outcome} | {sd:.2f} / {sj:.2f} | {td:.2f} / {tj:.2f} | {rd:.2f} / {rj:.2f} |".format(
                spec=row["spec_id"],
                version=version,
                outcome=row["outcome"],
                sd=float(det.get("spec_compliance") or 0),
                sj=float(judge["spec_compliance"]["score"]),
                td=float(det.get("tool_selection") or 0),
                tj=float(judge["tool_selection"]["score"]),
                rd=float(det.get("error_recovery") or 0),
                rj=float(judge["error_recovery"]["score"]),
            )
        )

    lines.extend(["", "## Judge notes on failures", ""])
    failures = [row for row in rows if row["outcome"] != "success"]
    if not failures:
        lines.append("No failing runs.")
    for row in failures:
        lines.append(
            f"### {row['spec_id']} ({row['model']} / prompt {row['prompt_version']} / schema {row['tool_schema_version']})"
        )
        lines.append("")
        modes = ", ".join(row["failure_modes"]) or "none"
        lines.append(f"Failure modes: {modes}.")
        lines.append("")
        for dimension, label in (
            ("spec_compliance", "Spec compliance"),
            ("tool_selection", "Tool selection"),
            ("error_recovery", "Error recovery"),
        ):
            lines.append(f"- {label}: {row['judge'][dimension]['reason']}")
        lines.append("")

    lines.extend(
        [
            "## What the two layers are for",
            "",
            "The deterministic columns fail when a route, scope flag, tool branch, build, or recovery step is wrong.",
            "The judge columns explain the same run in a sentence a person can read, and they are the path that calls a hosted model when EVAL_LIVE=1.",
            "",
        ]
    )
    return "\n".join(lines) + "\n"


if __name__ == "__main__":
    main()
