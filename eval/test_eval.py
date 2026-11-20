import json
import unittest
from pathlib import Path

from eval.metrics import build_chain, judge_trajectory, score_dimension


MINI = {
    "spec_id": "todo-api",
    "spec": "I need a simple TODO app",
    "model": "gpt-4",
    "prompt_version": "v1",
    "tool_schema_version": "v1",
    "steps": [
        {"agent": "Project Manager", "state": "Discovery", "tool": "convert_user_input_to_goal", "success": True},
        {"agent": "Solutions Architect", "state": "Discovery", "tool": "print_project_scope", "success": True},
        {"agent": "Backend Developer", "state": "Working", "tool": "print_backend_webserver_code", "success": True},
        {"agent": "Backend Developer", "state": "Working", "tool": "print_improved_webserver_code", "success": True},
        {"agent": "Backend Developer", "state": "UnitTesting", "tool": "print_rest_api_endpoints", "success": True},
        {"agent": "Backend Developer", "state": "UnitTesting", "tool": "cargo_build", "success": True},
        {"agent": "Backend Developer", "state": "UnitTesting", "tool": "runtime_smoke", "success": True},
        {"agent": "Backend Developer", "state": "UnitTesting", "tool": "endpoint_test", "success": True},
    ],
    "factsheet": {
        "project_description": "build a website that stores todo tasks",
        "project_scope": {
            "is_crud_required": True,
            "is_user_login_and_logout": False,
            "is_external_urls_required": False,
        },
        "api_endpoint_schema": [
            {"route": "/health", "method": "get", "is_route_dynamic": "false"},
            {"route": "/todos", "method": "get", "is_route_dynamic": "false"},
        ],
    },
    "expected_goal_contains": "todo",
    "expected_scope": {
        "is_crud_required": True,
        "is_user_login_and_logout": False,
        "is_external_urls_required": False,
    },
    "expected_routes": ["/health", "/todos"],
    "expected_tools": [
        "convert_user_input_to_goal",
        "print_project_scope",
        "print_backend_webserver_code",
        "print_improved_webserver_code",
        "print_rest_api_endpoints",
        "cargo_build",
        "runtime_smoke",
        "endpoint_test",
    ],
    "outcome": "success",
    "agent_error": None,
}


class JudgeTests(unittest.TestCase):
    def test_rubric_scores_a_clean_multi_step_run(self) -> None:
        for dimension in ("spec_compliance", "tool_selection", "error_recovery"):
            score, reason = score_dimension(dimension, MINI)
            self.assertEqual(score, 1.0, reason)

    def test_deepeval_metric_uses_the_langchain_chain(self) -> None:
        judged = judge_trajectory(MINI, build_chain(live=False))
        self.assertEqual(judged["spec_compliance"]["score"], 1.0)
        self.assertIn("todo-api", judged["tool_selection"]["reason"])
        self.assertTrue(judged["error_recovery"]["success"])

    def test_recovery_failure_is_zero(self) -> None:
        broken = json.loads(json.dumps(MINI))
        broken["steps"] = [step for step in broken["steps"] if step["tool"] != "cargo_build"]
        broken["steps"].append({"tool": "cargo_build", "success": False})
        broken["agent_error"] = "Error: Too many bugs"
        score, reason = score_dimension("error_recovery", broken)
        self.assertEqual(score, 0.0, reason)
        self.assertIn("failed", reason)

    def test_sample_report_shape_if_present(self) -> None:
        path = Path("docs/sample_trajectories.json")
        if not path.exists():
            self.skipTest("sample trajectories are produced by the replay command")
        rows = json.loads(path.read_text())
        self.assertGreaterEqual(len(rows), 8)
        judged = judge_trajectory(rows[0], build_chain(live=False))
        self.assertIn("score", judged["spec_compliance"])


if __name__ == "__main__":
    unittest.main()
