# Trajectory replay report

The harness re-ran each benchmark spec through the Manager, Solutions Architect, and Backend Developer with a fixture model. Rows are compared with the baseline `gpt-4 / prompt v1 / schema v1`. Failure clusters group runs that missed in the same way.

Runs: 32. Failures: 10. Recovered build errors: 3. Regressions against baseline: 10.

## Matrix

| Spec | Model | Prompt | Schema | Outcome | Failure modes | Recovered | Spec | Tools | Recovery |
| --- | --- | --- | --- | --- | --- | --- | ---: | ---: | ---: |
| todo-api | gpt-4 | v1 | v1 | success | — | no | 1.00 | 1.00 | 1.00 |
| todo-api | gpt-4o | v1 | v1 | failure | spec_mismatch, tool_selection_error | no | 0.83 | 0.50 | 1.00 |
| todo-api | gpt-4 | v2 | v1 | success | — | no | 1.00 | 1.00 | 1.00 |
| todo-api | gpt-4 | v1 | v2 | success | — | no | 1.00 | 1.00 | 1.00 |
| weather-api | gpt-4 | v1 | v1 | success | — | no | 1.00 | 1.00 | 1.00 |
| weather-api | gpt-4o | v1 | v1 | failure | endpoint_failed | no | 1.00 | 1.00 | 1.00 |
| weather-api | gpt-4 | v2 | v1 | failure | endpoint_failed | no | 1.00 | 1.00 | 1.00 |
| weather-api | gpt-4 | v1 | v2 | failure | build_failed, error_recovery_failed | no | 1.00 | 0.63 | 0.00 |
| notes-api | gpt-4 | v1 | v1 | success | — | no | 1.00 | 1.00 | 1.00 |
| notes-api | gpt-4o | v1 | v1 | success | — | no | 1.00 | 1.00 | 1.00 |
| notes-api | gpt-4 | v2 | v1 | failure | spec_mismatch | no | 0.83 | 1.00 | 1.00 |
| notes-api | gpt-4 | v1 | v2 | success | — | no | 1.00 | 1.00 | 1.00 |
| forex-prices | gpt-4 | v1 | v1 | success | — | no | 1.00 | 1.00 | 1.00 |
| forex-prices | gpt-4o | v1 | v1 | failure | spec_mismatch, tool_selection_error | no | 0.83 | 0.80 | 1.00 |
| forex-prices | gpt-4 | v2 | v1 | success | — | no | 1.00 | 1.00 | 1.00 |
| forex-prices | gpt-4 | v1 | v2 | success | — | no | 1.00 | 1.00 | 1.00 |
| fitness-tracker | gpt-4 | v1 | v1 | success | — | no | 1.00 | 1.00 | 1.00 |
| fitness-tracker | gpt-4o | v1 | v1 | success | — | no | 1.00 | 1.00 | 1.00 |
| fitness-tracker | gpt-4 | v2 | v1 | failure | spec_mismatch | no | 0.83 | 1.00 | 1.00 |
| fitness-tracker | gpt-4 | v1 | v2 | success | — | no | 1.00 | 1.00 | 1.00 |
| health-dashboard | gpt-4 | v1 | v1 | success | — | no | 1.00 | 1.00 | 1.00 |
| health-dashboard | gpt-4o | v1 | v1 | success | — | no | 1.00 | 1.00 | 1.00 |
| health-dashboard | gpt-4 | v2 | v1 | success | — | no | 1.00 | 1.00 | 1.00 |
| health-dashboard | gpt-4 | v1 | v2 | failure | build_failed, error_recovery_failed | no | 1.00 | 0.63 | 0.00 |
| inventory-api | gpt-4 | v1 | v1 | success | — | no | 1.00 | 1.00 | 1.00 |
| inventory-api | gpt-4o | v1 | v1 | success | — | no | 1.00 | 1.00 | 1.00 |
| inventory-api | gpt-4 | v2 | v1 | success | — | no | 1.00 | 1.00 | 1.00 |
| inventory-api | gpt-4 | v1 | v2 | failure | tool_schema_mismatch | no | 1.00 | 0.88 | 1.00 |
| bookmarks-api | gpt-4 | v1 | v1 | success | — | yes | 1.00 | 1.00 | 1.00 |
| bookmarks-api | gpt-4o | v1 | v1 | success | — | yes | 1.00 | 1.00 | 1.00 |
| bookmarks-api | gpt-4 | v2 | v1 | success | — | yes | 1.00 | 1.00 | 1.00 |
| bookmarks-api | gpt-4 | v1 | v2 | failure | tool_schema_mismatch | no | 1.00 | 0.78 | 1.00 |

## Regressions against baseline

- `todo-api` on gpt-4o / prompt v1 / schema v1 went from `success` (none) to `failure` (spec_mismatch, tool_selection_error).
- `weather-api` on gpt-4o / prompt v1 / schema v1 went from `success` (none) to `failure` (endpoint_failed).
- `weather-api` on gpt-4 / prompt v2 / schema v1 went from `success` (none) to `failure` (endpoint_failed).
- `weather-api` on gpt-4 / prompt v1 / schema v2 went from `success` (none) to `failure` (build_failed, error_recovery_failed).
- `notes-api` on gpt-4 / prompt v2 / schema v1 went from `success` (none) to `failure` (spec_mismatch).
- `forex-prices` on gpt-4o / prompt v1 / schema v1 went from `success` (none) to `failure` (spec_mismatch, tool_selection_error).
- `fitness-tracker` on gpt-4 / prompt v2 / schema v1 went from `success` (none) to `failure` (spec_mismatch).
- `health-dashboard` on gpt-4 / prompt v1 / schema v2 went from `success` (none) to `failure` (build_failed, error_recovery_failed).
- `inventory-api` on gpt-4 / prompt v1 / schema v2 went from `success` (none) to `failure` (tool_schema_mismatch).
- `bookmarks-api` on gpt-4 / prompt v1 / schema v2 went from `success` (none) to `failure` (tool_schema_mismatch).

## Recovered errors

These runs failed `cargo build`, called `print_fixed_code`, and then built successfully.

- `bookmarks-api` on gpt-4 / prompt v1 / schema v1
- `bookmarks-api` on gpt-4o / prompt v1 / schema v1
- `bookmarks-api` on gpt-4 / prompt v2 / schema v1

## Failure clusters

### build_failed+error_recovery_failed | E0308 (2 runs)

Signature: `E0308`.

- weather-api @ gpt-4 / prompt v1 / schema v2
- health-dashboard @ gpt-4 / prompt v1 / schema v2

Example:

```
Error: Too many bugs
```

### endpoint_failed | /weather (2 runs)

Signature: `/weather`.

- weather-api @ gpt-4o / prompt v1 / schema v1
- weather-api @ gpt-4 / prompt v2 / schema v1

Example:

```
/weather -> Some(500) 
```

### spec_mismatch | is_user_login_and_logout (2 runs)

Signature: `is_user_login_and_logout`.

- notes-api @ gpt-4 / prompt v2 / schema v1
- fitness-tracker @ gpt-4 / prompt v2 / schema v1

Example:

```
spec_mismatch
```

### spec_mismatch+tool_selection_error | is_external_urls_required (2 runs)

Signature: `is_external_urls_required`.

- todo-api @ gpt-4o / prompt v1 / schema v1
- forex-prices @ gpt-4o / prompt v1 / schema v1

Example:

```
spec_mismatch, tool_selection_error
```

### tool_schema_mismatch | /health (2 runs)

Signature: `/health`.

- inventory-api @ gpt-4 / prompt v1 / schema v2
- bookmarks-api @ gpt-4 / prompt v1 / schema v2

Example:

```
tool_schema_mismatch
```

## How to read a cluster

- `build_failed` + `error_recovery_failed`: the compiler rejected the service and `print_fixed_code` did not repair it.
- `endpoint_failed`: the service started, and a static GET route did not return 200.
- `tool_schema_mismatch`: the route list used a method or dynamic flag the endpoint probe does not call (tool schema v1 expects lowercase `get` and `is_route_dynamic` of `"false"`).
- `spec_mismatch`: the goal or project scope drifted from the benchmark expectation.
- `tool_selection_error`: the architect took a different external-URL branch than the spec calls for, so `print_site_urls` or `check_status_code` was called or skipped.
- `smoke_failed`: the binary did not answer HTTP on `/health`.
- `llm_decode_error`: a tool that must return JSON returned text the agent could not parse.
