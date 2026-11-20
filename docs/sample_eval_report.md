# Evaluation report

Deterministic checks come from the Rust harness (build, smoke, endpoints, scope, tool trace, recovery).
The judge is a DeepEval metric per axis, prompted with LangChain. This run used the offline rubric model behind a LangChain chain.
A judge score below 0.99 means that axis did not fully match the rubric.

## Scores

| Spec | Version | Outcome | Spec det / judge | Tools det / judge | Recovery det / judge |
| --- | --- | --- | --- | --- | --- |
| todo-api | gpt-4 / v1 / v1 | success | 1.00 / 1.00 | 1.00 / 1.00 | 1.00 / 1.00 |
| todo-api | gpt-4o / v1 / v1 | failure | 0.83 / 0.83 | 0.50 / 0.50 | 1.00 / 1.00 |
| todo-api | gpt-4 / v2 / v1 | success | 1.00 / 1.00 | 1.00 / 1.00 | 1.00 / 1.00 |
| todo-api | gpt-4 / v1 / v2 | success | 1.00 / 1.00 | 1.00 / 1.00 | 1.00 / 1.00 |
| weather-api | gpt-4 / v1 / v1 | success | 1.00 / 1.00 | 1.00 / 1.00 | 1.00 / 1.00 |
| weather-api | gpt-4o / v1 / v1 | failure | 1.00 / 1.00 | 1.00 / 1.00 | 1.00 / 1.00 |
| weather-api | gpt-4 / v2 / v1 | failure | 1.00 / 1.00 | 1.00 / 1.00 | 1.00 / 1.00 |
| weather-api | gpt-4 / v1 / v2 | failure | 1.00 / 1.00 | 0.63 / 0.63 | 0.00 / 0.00 |
| notes-api | gpt-4 / v1 / v1 | success | 1.00 / 1.00 | 1.00 / 1.00 | 1.00 / 1.00 |
| notes-api | gpt-4o / v1 / v1 | success | 1.00 / 1.00 | 1.00 / 1.00 | 1.00 / 1.00 |
| notes-api | gpt-4 / v2 / v1 | failure | 0.83 / 0.83 | 1.00 / 1.00 | 1.00 / 1.00 |
| notes-api | gpt-4 / v1 / v2 | success | 1.00 / 1.00 | 1.00 / 1.00 | 1.00 / 1.00 |
| forex-prices | gpt-4 / v1 / v1 | success | 1.00 / 1.00 | 1.00 / 1.00 | 1.00 / 1.00 |
| forex-prices | gpt-4o / v1 / v1 | failure | 0.83 / 0.83 | 0.80 / 0.80 | 1.00 / 1.00 |
| forex-prices | gpt-4 / v2 / v1 | success | 1.00 / 1.00 | 1.00 / 1.00 | 1.00 / 1.00 |
| forex-prices | gpt-4 / v1 / v2 | success | 1.00 / 1.00 | 1.00 / 1.00 | 1.00 / 1.00 |
| fitness-tracker | gpt-4 / v1 / v1 | success | 1.00 / 1.00 | 1.00 / 1.00 | 1.00 / 1.00 |
| fitness-tracker | gpt-4o / v1 / v1 | success | 1.00 / 1.00 | 1.00 / 1.00 | 1.00 / 1.00 |
| fitness-tracker | gpt-4 / v2 / v1 | failure | 0.83 / 0.83 | 1.00 / 1.00 | 1.00 / 1.00 |
| fitness-tracker | gpt-4 / v1 / v2 | success | 1.00 / 1.00 | 1.00 / 1.00 | 1.00 / 1.00 |
| health-dashboard | gpt-4 / v1 / v1 | success | 1.00 / 1.00 | 1.00 / 1.00 | 1.00 / 1.00 |
| health-dashboard | gpt-4o / v1 / v1 | success | 1.00 / 1.00 | 1.00 / 1.00 | 1.00 / 1.00 |
| health-dashboard | gpt-4 / v2 / v1 | success | 1.00 / 1.00 | 1.00 / 1.00 | 1.00 / 1.00 |
| health-dashboard | gpt-4 / v1 / v2 | failure | 1.00 / 1.00 | 0.63 / 0.63 | 0.00 / 0.00 |
| inventory-api | gpt-4 / v1 / v1 | success | 1.00 / 1.00 | 1.00 / 1.00 | 1.00 / 1.00 |
| inventory-api | gpt-4o / v1 / v1 | success | 1.00 / 1.00 | 1.00 / 1.00 | 1.00 / 1.00 |
| inventory-api | gpt-4 / v2 / v1 | success | 1.00 / 1.00 | 1.00 / 1.00 | 1.00 / 1.00 |
| inventory-api | gpt-4 / v1 / v2 | failure | 1.00 / 1.00 | 0.88 / 0.88 | 1.00 / 1.00 |
| bookmarks-api | gpt-4 / v1 / v1 | success | 1.00 / 1.00 | 1.00 / 1.00 | 1.00 / 1.00 |
| bookmarks-api | gpt-4o / v1 / v1 | success | 1.00 / 1.00 | 1.00 / 1.00 | 1.00 / 1.00 |
| bookmarks-api | gpt-4 / v2 / v1 | success | 1.00 / 1.00 | 1.00 / 1.00 | 1.00 / 1.00 |
| bookmarks-api | gpt-4 / v1 / v2 | failure | 1.00 / 1.00 | 0.78 / 0.78 | 1.00 / 1.00 |

## Judge notes on failures

### todo-api (gpt-4o / prompt v1 / schema v1)

Failure modes: spec_mismatch, tool_selection_error.

- Spec compliance: todo-api: spec drift at is_external_urls_required was True
- Tool selection: todo-api: unexpected planning tools check_status_code, print_site_urls
- Error recovery: todo-api: cargo_build succeeded on the first attempt

### weather-api (gpt-4o / prompt v1 / schema v1)

Failure modes: endpoint_failed.

- Spec compliance: weather-api: scope, goal, and routes match the spec across the run
- Tool selection: weather-api: tool trace matches the expected branch, including every multi-step call
- Error recovery: weather-api: cargo_build succeeded on the first attempt

### weather-api (gpt-4 / prompt v2 / schema v1)

Failure modes: endpoint_failed.

- Spec compliance: weather-api: scope, goal, and routes match the spec across the run
- Tool selection: weather-api: tool trace matches the expected branch, including every multi-step call
- Error recovery: weather-api: cargo_build succeeded on the first attempt

### weather-api (gpt-4 / prompt v1 / schema v2)

Failure modes: build_failed, error_recovery_failed.

- Spec compliance: weather-api: scope, goal, and routes match the spec across the run
- Tool selection: weather-api: missing tools print_rest_api_endpoints, runtime_smoke, endpoint_test
- Error recovery: weather-api: every cargo_build failed; print_fixed_code did not recover

### notes-api (gpt-4 / prompt v2 / schema v1)

Failure modes: spec_mismatch.

- Spec compliance: notes-api: spec drift at is_user_login_and_logout was False
- Tool selection: notes-api: tool trace matches the expected branch, including every multi-step call
- Error recovery: notes-api: cargo_build succeeded on the first attempt

### forex-prices (gpt-4o / prompt v1 / schema v1)

Failure modes: spec_mismatch, tool_selection_error.

- Spec compliance: forex-prices: spec drift at is_external_urls_required was False
- Tool selection: forex-prices: missing tools print_site_urls, check_status_code
- Error recovery: forex-prices: cargo_build succeeded on the first attempt

### fitness-tracker (gpt-4 / prompt v2 / schema v1)

Failure modes: spec_mismatch.

- Spec compliance: fitness-tracker: spec drift at is_user_login_and_logout was False
- Tool selection: fitness-tracker: tool trace matches the expected branch, including every multi-step call
- Error recovery: fitness-tracker: cargo_build succeeded on the first attempt

### health-dashboard (gpt-4 / prompt v1 / schema v2)

Failure modes: build_failed, error_recovery_failed.

- Spec compliance: health-dashboard: scope, goal, and routes match the spec across the run
- Tool selection: health-dashboard: missing tools print_rest_api_endpoints, runtime_smoke, endpoint_test
- Error recovery: health-dashboard: every cargo_build failed; print_fixed_code did not recover

### inventory-api (gpt-4 / prompt v1 / schema v2)

Failure modes: tool_schema_mismatch.

- Spec compliance: inventory-api: scope, goal, and routes match the spec across the run
- Tool selection: inventory-api: missing tools endpoint_test
- Error recovery: inventory-api: cargo_build succeeded on the first attempt

### bookmarks-api (gpt-4 / prompt v1 / schema v2)

Failure modes: tool_schema_mismatch.

- Spec compliance: bookmarks-api: scope, goal, and routes match the spec across the run
- Tool selection: bookmarks-api: missing tools print_fixed_code, endpoint_test
- Error recovery: bookmarks-api: cargo_build succeeded on the first attempt

## What the two layers are for

The deterministic columns fail when a route, scope flag, tool branch, build, or recovery step is wrong.
The judge columns explain the same run in a sentence a person can read, and they are the path that calls a hosted model when EVAL_LIVE=1.

