//! Deterministic scores and failure labels for a recorded trajectory.
//!
//! These checks do not call a model. The Python package in `eval/` judges the
//! same three axes (spec compliance, tool selection, error recovery) with
//! DeepEval and reports both numbers side by side.

use crate::models::agents::agent_traits::{ProjectScope, RouteObject};
use crate::trajectory::{DeterministicScores, ToolStep, Trajectory};
use crate::verification::{VerificationReport, rustc_error_code};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const PLANNING_TOOLS: &[&str] = &["print_site_urls", "check_status_code"];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpectedSpec {
    pub id: String,
    pub request: String,
    pub expected_goal_contains: String,
    pub expected_scope: ProjectScope,
    pub expected_routes: Vec<String>,
    pub expected_tools: Vec<String>,
}

pub fn finalize(mut trajectory: Trajectory, spec: &ExpectedSpec) -> Trajectory {
    trajectory.expected_goal_contains = spec.expected_goal_contains.clone();
    trajectory.expected_scope = Some(spec.expected_scope);
    trajectory.expected_routes = spec.expected_routes.clone();
    trajectory.expected_tools = spec.expected_tools.clone();

    let analysis = analyze(&trajectory, spec);
    trajectory.outcome = if analysis.modes.is_empty() {
        "success".to_string()
    } else {
        "failure".to_string()
    };
    trajectory.failure_modes = analysis.modes;
    trajectory.signature = analysis.signature;
    trajectory.recovered = analysis.recovered;
    trajectory.deterministic = Some(analysis.scores);
    trajectory
}

struct Analysis {
    modes: Vec<String>,
    signature: String,
    recovered: bool,
    scores: DeterministicScores,
}

fn analyze(trajectory: &Trajectory, spec: &ExpectedSpec) -> Analysis {
    let mut notes = Vec::new();
    let spec_score = spec_compliance(trajectory, spec, &mut notes);
    let tool_score = tool_selection_score(&tool_names(&trajectory.steps), &spec.expected_tools);
    let recovery_score = error_recovery_score(&trajectory.steps, trajectory.agent_error.as_deref());
    let recovered = recovered_from_build_failure(&trajectory.steps);

    let build_passed = build_succeeded(trajectory);
    let smoke_passed = trajectory
        .verification
        .as_ref()
        .and_then(|report| report.smoke.as_ref())
        .map(|smoke| smoke.success)
        .unwrap_or(false);
    let endpoints_passed = trajectory
        .verification
        .as_ref()
        .map(|report| {
            !report.endpoints.is_empty() && report.endpoints.iter().all(|check| check.success)
        })
        .unwrap_or(false);

    let mut modes = Vec::new();
    if decode_failed(trajectory) {
        modes.push("llm_decode_error".to_string());
    }
    let build_failed = build_attempted(trajectory) && !build_passed;
    if build_failed {
        modes.push("build_failed".to_string());
    }
    if build_failed && !recovered {
        modes.push("error_recovery_failed".to_string());
    }
    if let Some(report) = &trajectory.verification {
        if report.smoke.as_ref().is_some_and(|smoke| !smoke.success) {
            modes.push("smoke_failed".to_string());
        }
        if report.endpoints.iter().any(|check| !check.success) {
            modes.push("endpoint_failed".to_string());
        }
    }
    if planning_tools_differ(&tool_names(&trajectory.steps), &spec.expected_tools) {
        modes.push("tool_selection_error".to_string());
    }
    if spec_score.mismatched {
        modes.push("spec_mismatch".to_string());
    }
    if build_passed && schema_blocked(trajectory, spec) {
        modes.push("tool_schema_mismatch".to_string());
    }
    modes.sort();
    modes.dedup();

    let signature = signature_for(trajectory, spec, &modes);
    Analysis {
        modes,
        signature,
        recovered,
        scores: DeterministicScores {
            spec_compliance: round2(spec_score.score),
            tool_selection: round2(tool_score),
            error_recovery: round2(recovery_score),
            build_passed,
            smoke_passed,
            endpoints_passed,
            notes,
        },
    }
}

struct SpecScore {
    score: f64,
    mismatched: bool,
    first_mismatch: String,
}

fn spec_compliance(
    trajectory: &Trajectory,
    spec: &ExpectedSpec,
    notes: &mut Vec<String>,
) -> SpecScore {
    let mut points = 0.0;
    let mut total = 0.0;
    let mut mismatched = false;
    let mut first_mismatch = String::new();

    let note_mismatch =
        |field: &str, mismatched: &mut bool, first: &mut String, notes: &mut Vec<String>| {
            *mismatched = true;
            if first.is_empty() {
                *first = field.to_string();
            }
            notes.push(format!("spec field {field} did not match"));
        };

    total += 1.0;
    let goal = trajectory
        .factsheet
        .as_ref()
        .map(|sheet| sheet.project_description.to_lowercase())
        .unwrap_or_default();
    let needle = spec.expected_goal_contains.to_lowercase();
    if !needle.is_empty() && goal.contains(&needle) {
        points += 1.0;
    } else if !needle.is_empty() {
        note_mismatch("goal", &mut mismatched, &mut first_mismatch, notes);
    } else {
        points += 1.0;
    }

    total += 3.0;
    match (
        trajectory
            .factsheet
            .as_ref()
            .and_then(|sheet| sheet.project_scope),
        Some(spec.expected_scope),
    ) {
        (Some(actual), Some(expected)) => {
            for (field, ok) in [
                (
                    "is_crud_required",
                    actual.is_crud_required == expected.is_crud_required,
                ),
                (
                    "is_user_login_and_logout",
                    actual.is_user_login_and_logout == expected.is_user_login_and_logout,
                ),
                (
                    "is_external_urls_required",
                    actual.is_external_urls_required == expected.is_external_urls_required,
                ),
            ] {
                if ok {
                    points += 1.0;
                } else {
                    note_mismatch(field, &mut mismatched, &mut first_mismatch, notes);
                }
            }
        }
        _ => {
            note_mismatch("project_scope", &mut mismatched, &mut first_mismatch, notes);
        }
    }

    let schema = schema_routes(
        trajectory
            .factsheet
            .as_ref()
            .and_then(|sheet| sheet.api_endpoint_schema.as_ref()),
    );
    let build_ok = build_succeeded(trajectory);
    for route in &spec.expected_routes {
        total += 1.0;
        if schema.iter().any(|item| item.route == *route) {
            points += 1.0;
        } else if build_ok {
            let field = format!("route:{route}");
            note_mismatch(&field, &mut mismatched, &mut first_mismatch, notes);
        } else {
            // The route list was never extracted because the build did not succeed.
            points += 1.0;
        }
    }

    SpecScore {
        score: if total == 0.0 { 1.0 } else { points / total },
        mismatched,
        first_mismatch,
    }
}

fn schema_routes(schema: Option<&Vec<RouteObject>>) -> Vec<RouteObject> {
    schema.cloned().unwrap_or_default()
}

pub fn tool_selection_score(actual: &[String], expected: &[String]) -> f64 {
    if expected.is_empty() {
        return 1.0;
    }
    let actual_set: HashSet<&str> = actual.iter().map(String::as_str).collect();
    let hits = expected
        .iter()
        .filter(|tool| actual_set.contains(tool.as_str()))
        .count();
    let recall = hits as f64 / expected.len() as f64;
    let expected_set: HashSet<&str> = expected.iter().map(String::as_str).collect();
    let unexpected = actual
        .iter()
        .filter(|tool| {
            PLANNING_TOOLS.contains(&tool.as_str()) && !expected_set.contains(tool.as_str())
        })
        .collect::<HashSet<_>>()
        .len();
    (recall - 0.25 * unexpected as f64).clamp(0.0, 1.0)
}

fn planning_tools_differ(actual: &[String], expected: &[String]) -> bool {
    let actual_set: HashSet<&str> = actual.iter().map(String::as_str).collect();
    let expected_set: HashSet<&str> = expected.iter().map(String::as_str).collect();
    PLANNING_TOOLS
        .iter()
        .any(|tool| actual_set.contains(tool) != expected_set.contains(tool))
}

pub fn error_recovery_score(steps: &[ToolStep], agent_error: Option<&str>) -> f64 {
    let builds: Vec<&ToolStep> = steps
        .iter()
        .filter(|step| step.tool == "cargo_build")
        .collect();
    if builds.is_empty() {
        return if agent_error.is_some() { 0.0 } else { 1.0 };
    }
    if builds.iter().any(|step| step.success) {
        1.0
    } else {
        0.0
    }
}

fn recovered_from_build_failure(steps: &[ToolStep]) -> bool {
    let mut seen_failure = false;
    for step in steps.iter().filter(|step| step.tool == "cargo_build") {
        if !step.success {
            seen_failure = true;
        } else if seen_failure {
            return true;
        }
    }
    false
}

fn build_attempted(trajectory: &Trajectory) -> bool {
    trajectory
        .steps
        .iter()
        .any(|step| step.tool == "cargo_build")
        || trajectory.verification.as_ref().is_some()
}

fn build_succeeded(trajectory: &Trajectory) -> bool {
    if let Some(report) = &trajectory.verification
        && report.build.success
    {
        return true;
    }
    trajectory
        .steps
        .iter()
        .rev()
        .find(|step| step.tool == "cargo_build")
        .is_some_and(|step| step.success)
}

fn decode_failed(trajectory: &Trajectory) -> bool {
    let agent_says = trajectory
        .agent_error
        .as_ref()
        .is_some_and(|error| error.to_lowercase().contains("decode"));
    let step_says = trajectory.steps.iter().any(|step| {
        step.error
            .as_ref()
            .is_some_and(|error| error.to_lowercase().contains("decode"))
    });
    agent_says || step_says
}

fn schema_blocked(trajectory: &Trajectory, spec: &ExpectedSpec) -> bool {
    let Some(schema) = trajectory
        .factsheet
        .as_ref()
        .and_then(|sheet| sheet.api_endpoint_schema.as_ref())
    else {
        return false;
    };
    let tested = tested_routes(trajectory.verification.as_ref());
    spec.expected_routes.iter().any(|route| {
        schema.iter().any(|item| {
            item.route == *route
                && (item.method != "get" || item.is_route_dynamic != "false")
                && !tested.contains(route)
        })
    })
}

fn tested_routes(report: Option<&VerificationReport>) -> HashSet<String> {
    report
        .map(|report| {
            report
                .endpoints
                .iter()
                .map(|check| check.route.clone())
                .collect()
        })
        .unwrap_or_default()
}

fn tool_names(steps: &[ToolStep]) -> Vec<String> {
    steps.iter().map(|step| step.tool.clone()).collect()
}

fn signature_for(trajectory: &Trajectory, spec: &ExpectedSpec, modes: &[String]) -> String {
    if modes.is_empty() {
        return String::new();
    }
    if modes.iter().any(|mode| mode == "build_failed") {
        if let Some(code) = trajectory
            .verification
            .as_ref()
            .and_then(|report| rustc_error_code(&report.build.stderr))
        {
            return code;
        }
        if let Some(code) = trajectory
            .steps
            .iter()
            .rev()
            .filter(|step| step.tool == "cargo_build" && !step.success)
            .find_map(|step| rustc_error_code(&step.output_excerpt))
        {
            return code;
        }
        return "build".to_string();
    }
    if modes.iter().any(|mode| mode == "endpoint_failed")
        && let Some(route) = trajectory.verification.as_ref().and_then(|report| {
            report
                .endpoints
                .iter()
                .find(|check| !check.success)
                .map(|check| check.route.clone())
        })
    {
        return route;
    }
    if modes.iter().any(|mode| mode == "tool_schema_mismatch")
        && let Some(route) = first_blocked_route(trajectory, spec)
    {
        return route;
    }
    if modes.iter().any(|mode| mode == "spec_mismatch") {
        let mut notes = Vec::new();
        let score = spec_compliance(trajectory, spec, &mut notes);
        if !score.first_mismatch.is_empty() {
            return score.first_mismatch;
        }
    }
    if modes.iter().any(|mode| mode == "tool_selection_error") {
        return "planning_tools".to_string();
    }
    if modes.iter().any(|mode| mode == "smoke_failed") {
        return "smoke".to_string();
    }
    if modes.iter().any(|mode| mode == "llm_decode_error") {
        return "decode".to_string();
    }
    modes.first().cloned().unwrap_or_default()
}

fn first_blocked_route(trajectory: &Trajectory, spec: &ExpectedSpec) -> Option<String> {
    let schema = trajectory
        .factsheet
        .as_ref()
        .and_then(|sheet| sheet.api_endpoint_schema.as_ref())?;
    let tested = tested_routes(trajectory.verification.as_ref());
    spec.expected_routes
        .iter()
        .find(|route| {
            schema.iter().any(|item| {
                item.route == **route
                    && (item.method != "get" || item.is_route_dynamic != "false")
                    && !tested.contains(*route)
            })
        })
        .cloned()
}

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::agents::agent_traits::FactSheet;
    use crate::trajectory::ToolStep;
    use crate::verification::{BuildReport, EndpointCheck, SmokeReport, VerificationReport};
    use serde_json::json;

    fn step(tool: &str, success: bool) -> ToolStep {
        ToolStep {
            agent: "Backend Developer".into(),
            state: "UnitTesting".into(),
            tool: tool.into(),
            input_excerpt: String::new(),
            output_excerpt: String::new(),
            success,
            error: None,
            duration_ms: 1,
        }
    }

    fn spec_todo() -> ExpectedSpec {
        ExpectedSpec {
            id: "todo-api".into(),
            request: "todo".into(),
            expected_goal_contains: "todo".into(),
            expected_scope: ProjectScope {
                is_crud_required: true,
                is_user_login_and_logout: false,
                is_external_urls_required: false,
            },
            expected_routes: vec!["/health".into(), "/todos".into()],
            expected_tools: vec![
                "convert_user_input_to_goal".into(),
                "print_project_scope".into(),
                "print_backend_webserver_code".into(),
                "cargo_build".into(),
            ],
        }
    }

    fn passing_sheet() -> FactSheet {
        FactSheet {
            project_description: "build a website that stores todo tasks".into(),
            project_scope: Some(ProjectScope {
                is_crud_required: true,
                is_user_login_and_logout: false,
                is_external_urls_required: false,
            }),
            external_urls: None,
            backend_code: Some("fn main() {}".into()),
            api_endpoint_schema: Some(vec![route("/health", "get"), route("/todos", "get")]),
        }
    }

    fn route(path: &str, method: &str) -> RouteObject {
        RouteObject {
            is_route_dynamic: "false".into(),
            method: method.into(),
            request_body: json!("None"),
            response: json!({"ok": "bool"}),
            route: path.into(),
        }
    }

    #[test]
    fn tool_selection_penalizes_an_unexpected_planning_tool() {
        let actual = vec![
            "convert_user_input_to_goal".into(),
            "print_project_scope".into(),
            "print_site_urls".into(),
            "print_backend_webserver_code".into(),
            "cargo_build".into(),
        ];
        let expected = vec![
            "convert_user_input_to_goal".into(),
            "print_project_scope".into(),
            "print_backend_webserver_code".into(),
            "cargo_build".into(),
        ];
        let score = tool_selection_score(&actual, &expected);
        assert!(score < 1.0, "{score}");
        assert!(planning_tools_differ(&actual, &expected));
    }

    #[test]
    fn recovery_score_is_one_after_a_later_successful_build() {
        let steps = vec![
            step("cargo_build", false),
            step("print_fixed_code", true),
            step("cargo_build", true),
        ];
        assert_eq!(error_recovery_score(&steps, None), 1.0);
        assert!(recovered_from_build_failure(&steps));
    }

    #[test]
    fn recovery_score_is_zero_when_every_build_fails() {
        let steps = vec![step("cargo_build", false), step("cargo_build", false)];
        assert_eq!(error_recovery_score(&steps, None), 0.0);
        assert!(!recovered_from_build_failure(&steps));
    }

    #[test]
    fn finalize_flags_scope_and_planning_regressions() {
        let spec = spec_todo();
        let mut trajectory = Trajectory::new("todo-api", "todo", "gpt-4o", "v1", "v1");
        trajectory.steps = vec![
            step("convert_user_input_to_goal", true),
            step("print_project_scope", true),
            step("print_site_urls", true),
            step("check_status_code", true),
            step("print_backend_webserver_code", true),
            step("cargo_build", true),
        ];
        let mut sheet = passing_sheet();
        sheet.project_scope = Some(ProjectScope {
            is_crud_required: true,
            is_user_login_and_logout: false,
            is_external_urls_required: true,
        });
        trajectory.factsheet = Some(sheet);
        trajectory.verification = Some(VerificationReport {
            build: BuildReport {
                success: true,
                stdout: String::new(),
                stderr: String::new(),
            },
            smoke: Some(SmokeReport {
                success: true,
                detail: "ok".into(),
                startup_ms: 10,
            }),
            endpoints: vec![EndpointCheck {
                route: "/health".into(),
                url: "http://127.0.0.1/health".into(),
                expected_status: 200,
                actual_status: Some(200),
                success: true,
                error: None,
            }],
        });
        let done = finalize(trajectory, &spec);
        assert_eq!(done.outcome, "failure");
        assert!(done.failure_modes.contains(&"spec_mismatch".to_string()));
        assert!(
            done.failure_modes
                .contains(&"tool_selection_error".to_string())
        );
        assert_eq!(done.signature, "is_external_urls_required");
    }

    #[test]
    fn finalize_clusters_unrepaired_builds_on_the_rustc_code() {
        let mut spec = spec_todo();
        spec.expected_goal_contains = "weather".into();
        spec.expected_routes.clear();
        let mut trajectory = Trajectory::new("weather-api", "weather", "gpt-4", "v1", "v2");
        let mut failed = step("cargo_build", false);
        failed.output_excerpt = "error[E0308]: mismatched types".into();
        trajectory.steps = vec![failed];
        trajectory.verification = Some(VerificationReport {
            build: BuildReport {
                success: false,
                stdout: String::new(),
                stderr: "error[E0308]: mismatched types".into(),
            },
            smoke: None,
            endpoints: Vec::new(),
        });
        trajectory.factsheet = Some(FactSheet {
            project_description: "build a website that returns weather".into(),
            project_scope: Some(spec.expected_scope),
            external_urls: None,
            backend_code: None,
            api_endpoint_schema: None,
        });
        let done = finalize(trajectory, &spec);
        assert!(done.failure_modes.contains(&"build_failed".to_string()));
        assert!(
            done.failure_modes
                .contains(&"error_recovery_failed".to_string())
        );
        assert_eq!(done.signature, "E0308");
        assert!(!done.failure_modes.contains(&"spec_mismatch".to_string()));
    }
}
