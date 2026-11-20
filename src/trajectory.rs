//! Recorded agent trajectories: one step per model call and verification tool.
//!
//! Replay writes these documents and diffs them across model, prompt, and
//! tool-schema versions. The Python judge reads the same JSON.

use crate::models::agents::agent_traits::{FactSheet, ProjectScope};
use crate::run_context;
use crate::verification::VerificationReport;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

tokio::task_local! {
    static TRAJ: Arc<Mutex<Trajectory>>;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DeterministicScores {
    pub spec_compliance: f64,
    pub tool_selection: f64,
    pub error_recovery: f64,
    pub build_passed: bool,
    pub smoke_passed: bool,
    pub endpoints_passed: bool,
    pub notes: Vec<String>,
}

impl Default for DeterministicScores {
    fn default() -> Self {
        Self {
            spec_compliance: 0.0,
            tool_selection: 0.0,
            error_recovery: 0.0,
            build_passed: false,
            smoke_passed: false,
            endpoints_passed: false,
            notes: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolStep {
    pub agent: String,
    pub state: String,
    pub tool: String,
    pub input_excerpt: String,
    pub output_excerpt: String,
    pub success: bool,
    pub error: Option<String>,
    pub duration_ms: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trajectory {
    pub spec_id: String,
    pub spec: String,
    pub model: String,
    pub prompt_version: String,
    pub tool_schema_version: String,
    pub steps: Vec<ToolStep>,
    pub factsheet: Option<FactSheet>,
    pub verification: Option<VerificationReport>,
    pub agent_error: Option<String>,
    pub expected_goal_contains: String,
    pub expected_scope: Option<ProjectScope>,
    pub expected_routes: Vec<String>,
    pub expected_tools: Vec<String>,
    pub outcome: String,
    pub failure_modes: Vec<String>,
    pub signature: String,
    pub recovered: bool,
    pub deterministic: Option<DeterministicScores>,
}

impl Trajectory {
    pub fn new(
        spec_id: impl Into<String>,
        spec: impl Into<String>,
        model: impl Into<String>,
        prompt_version: impl Into<String>,
        tool_schema_version: impl Into<String>,
    ) -> Self {
        Self {
            spec_id: spec_id.into(),
            spec: spec.into(),
            model: model.into(),
            prompt_version: prompt_version.into(),
            tool_schema_version: tool_schema_version.into(),
            steps: Vec::new(),
            factsheet: None,
            verification: None,
            agent_error: None,
            expected_goal_contains: String::new(),
            expected_scope: None,
            expected_routes: Vec::new(),
            expected_tools: Vec::new(),
            outcome: "pending".to_string(),
            failure_modes: Vec::new(),
            signature: String::new(),
            recovered: false,
            deterministic: None,
        }
    }

    pub fn version_label(&self) -> String {
        format!(
            "{} / prompt {} / schema {}",
            self.model, self.prompt_version, self.tool_schema_version
        )
    }
}

pub async fn scope<F>(trajectory: Arc<Mutex<Trajectory>>, future: F) -> F::Output
where
    F: std::future::Future,
{
    TRAJ.scope(trajectory, future).await
}

pub fn record_tool(
    agent: &str,
    tool: &str,
    input: &str,
    output: &str,
    success: bool,
    error: Option<String>,
    duration_ms: u128,
) {
    let step = ToolStep {
        agent: agent.to_string(),
        state: run_context::phase(),
        tool: tool.to_string(),
        input_excerpt: crate::llm::excerpt(input, 1500),
        output_excerpt: crate::llm::excerpt(output, 4000),
        success,
        error,
        duration_ms,
    };
    let _ = TRAJ.try_with(|trajectory| {
        trajectory.lock().expect("trajectory lock").steps.push(step);
    });
}

pub fn mark_last_step_failed(error: &str) {
    let _ = TRAJ.try_with(|trajectory| {
        let mut guard = trajectory.lock().expect("trajectory lock");
        if let Some(step) = guard.steps.last_mut() {
            step.success = false;
            step.error = Some(error.to_string());
        }
    });
}

pub fn set_verification(report: VerificationReport) {
    let _ = TRAJ.try_with(|trajectory| {
        trajectory.lock().expect("trajectory lock").verification = Some(report);
    });
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OutcomeDiff {
    pub spec_id: String,
    pub version: String,
    pub baseline_outcome: String,
    pub outcome: String,
    pub baseline_modes: Vec<String>,
    pub modes: Vec<String>,
    pub kind: String,
}

pub fn diff_against_baseline(runs: &[Trajectory], baseline: &Trajectory) -> Vec<OutcomeDiff> {
    runs.iter()
        .filter(|run| run.spec_id == baseline.spec_id)
        .filter(|run| !same_version(run, baseline))
        .map(|run| {
            let kind =
                if run.outcome == baseline.outcome && run.failure_modes == baseline.failure_modes {
                    "unchanged"
                } else if baseline.outcome == "success" && run.outcome != "success" {
                    "regression"
                } else if baseline.outcome != "success" && run.outcome == "success" {
                    "improvement"
                } else {
                    "changed"
                };
            OutcomeDiff {
                spec_id: run.spec_id.clone(),
                version: run.version_label(),
                baseline_outcome: baseline.outcome.clone(),
                outcome: run.outcome.clone(),
                baseline_modes: baseline.failure_modes.clone(),
                modes: run.failure_modes.clone(),
                kind: kind.to_string(),
            }
        })
        .collect()
}

pub fn same_version(left: &Trajectory, right: &Trajectory) -> bool {
    left.model == right.model
        && left.prompt_version == right.prompt_version
        && left.tool_schema_version == right.tool_schema_version
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FailureCluster {
    pub key: String,
    pub failure_modes: Vec<String>,
    pub signature: String,
    pub count: usize,
    pub runs: Vec<String>,
    pub example_error: String,
}

pub fn cluster_failures(runs: &[Trajectory]) -> Vec<FailureCluster> {
    let mut clusters: Vec<FailureCluster> = Vec::new();
    for run in runs.iter().filter(|run| run.outcome != "success") {
        let key = format!("{} | {}", run.failure_modes.join("+"), run.signature);
        if let Some(existing) = clusters.iter_mut().find(|cluster| cluster.key == key) {
            existing.count += 1;
            existing
                .runs
                .push(format!("{} @ {}", run.spec_id, run.version_label()));
        } else {
            clusters.push(FailureCluster {
                key,
                failure_modes: run.failure_modes.clone(),
                signature: run.signature.clone(),
                count: 1,
                runs: vec![format!("{} @ {}", run.spec_id, run.version_label())],
                example_error: example_error(run),
            });
        }
    }
    clusters.sort_by(|left, right| right.count.cmp(&left.count).then(left.key.cmp(&right.key)));
    clusters
}

fn example_error(run: &Trajectory) -> String {
    if let Some(error) = &run.agent_error {
        return crate::llm::excerpt(error, 500);
    }
    if let Some(report) = &run.verification {
        if !report.build.success && !report.build.stderr.is_empty() {
            return crate::llm::excerpt(&report.build.stderr, 500);
        }
        if let Some(check) = report.endpoints.iter().find(|check| !check.success) {
            return format!(
                "{} -> {:?} {}",
                check.route,
                check.actual_status,
                check.error.clone().unwrap_or_default()
            );
        }
    }
    run.failure_modes.join(", ")
}

pub fn render_report(
    runs: &[Trajectory],
    diffs: &[OutcomeDiff],
    clusters: &[FailureCluster],
) -> String {
    let mut out = String::new();
    out.push_str("# Trajectory replay report\n\n");
    out.push_str("The harness re-ran each benchmark spec through the Manager, Solutions Architect, and Backend Developer with a fixture model. ");
    out.push_str("Rows are compared with the baseline `gpt-4 / prompt v1 / schema v1`. ");
    out.push_str("Failure clusters group runs that missed in the same way.\n\n");

    let failures = runs.iter().filter(|run| run.outcome != "success").count();
    let recovered = runs.iter().filter(|run| run.recovered).count();
    let regressions = diffs
        .iter()
        .filter(|diff| diff.kind == "regression")
        .count();
    out.push_str(&format!(
        "Runs: {}. Failures: {failures}. Recovered build errors: {recovered}. Regressions against baseline: {regressions}.\n\n",
        runs.len()
    ));

    out.push_str("## Matrix\n\n");
    out.push_str("| Spec | Model | Prompt | Schema | Outcome | Failure modes | Recovered | Spec | Tools | Recovery |\n");
    out.push_str("| --- | --- | --- | --- | --- | --- | --- | ---: | ---: | ---: |\n");
    for run in runs {
        let scores = run.deterministic.clone().unwrap_or_default();
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {:.2} | {:.2} | {:.2} |\n",
            run.spec_id,
            run.model,
            run.prompt_version,
            run.tool_schema_version,
            run.outcome,
            if run.failure_modes.is_empty() {
                "—".to_string()
            } else {
                run.failure_modes.join(", ")
            },
            if run.recovered { "yes" } else { "no" },
            scores.spec_compliance,
            scores.tool_selection,
            scores.error_recovery,
        ));
    }

    out.push_str("\n## Regressions against baseline\n\n");
    let regression_rows: Vec<_> = diffs
        .iter()
        .filter(|diff| diff.kind == "regression")
        .collect();
    if regression_rows.is_empty() {
        out.push_str("No regressions.\n");
    } else {
        for diff in regression_rows {
            out.push_str(&format!(
                "- `{}` on {} went from `{}` ({}) to `{}` ({}).\n",
                diff.spec_id,
                diff.version,
                diff.baseline_outcome,
                join_or_dash(&diff.baseline_modes),
                diff.outcome,
                join_or_dash(&diff.modes),
            ));
        }
    }

    out.push_str("\n## Recovered errors\n\n");
    let recovered_runs: Vec<_> = runs.iter().filter(|run| run.recovered).collect();
    if recovered_runs.is_empty() {
        out.push_str("No run had to repair a failed build.\n");
    } else {
        out.push_str("These runs failed `cargo build`, called `print_fixed_code`, and then built successfully.\n\n");
        for run in recovered_runs {
            out.push_str(&format!("- `{}` on {}\n", run.spec_id, run.version_label()));
        }
    }

    out.push_str("\n## Failure clusters\n\n");
    if clusters.is_empty() {
        out.push_str("No failures to cluster.\n");
    }
    for cluster in clusters {
        out.push_str(&format!(
            "### {} ({} run{})\n\n",
            cluster.key,
            cluster.count,
            if cluster.count == 1 { "" } else { "s" }
        ));
        out.push_str(&format!("Signature: `{}`.\n\n", cluster.signature));
        for member in &cluster.runs {
            out.push_str(&format!("- {member}\n"));
        }
        if !cluster.example_error.is_empty() {
            out.push_str(&format!(
                "\nExample:\n\n```\n{}\n```\n",
                cluster.example_error
            ));
        }
        out.push('\n');
    }

    out.push_str("## How to read a cluster\n\n");
    out.push_str("- `build_failed` + `error_recovery_failed`: the compiler rejected the service and `print_fixed_code` did not repair it.\n");
    out.push_str(
        "- `endpoint_failed`: the service started, and a static GET route did not return 200.\n",
    );
    out.push_str("- `tool_schema_mismatch`: the route list used a method or dynamic flag the endpoint probe does not call (tool schema v1 expects lowercase `get` and `is_route_dynamic` of `\"false\"`).\n");
    out.push_str(
        "- `spec_mismatch`: the goal or project scope drifted from the benchmark expectation.\n",
    );
    out.push_str("- `tool_selection_error`: the architect took a different external-URL branch than the spec calls for, so `print_site_urls` or `check_status_code` was called or skipped.\n");
    out.push_str("- `smoke_failed`: the binary did not answer HTTP on `/health`.\n");
    out.push_str("- `llm_decode_error`: a tool that must return JSON returned text the agent could not parse.\n");
    out
}

fn join_or_dash(modes: &[String]) -> String {
    if modes.is_empty() {
        "none".to_string()
    } else {
        modes.join(", ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(spec: &str, modes: &[&str], signature: &str, outcome: &str) -> Trajectory {
        let mut trajectory = Trajectory::new(spec, spec, "gpt-4", "v1", "v2");
        trajectory.failure_modes = modes.iter().map(|mode| (*mode).to_string()).collect();
        trajectory.signature = signature.to_string();
        trajectory.outcome = outcome.to_string();
        trajectory
    }

    #[test]
    fn clusters_runs_that_share_a_signature() {
        let runs = vec![
            run(
                "weather-api",
                &["build_failed", "error_recovery_failed"],
                "E0308",
                "failure",
            ),
            run(
                "health-dashboard",
                &["build_failed", "error_recovery_failed"],
                "E0308",
                "failure",
            ),
            run("todo-api", &[], "", "success"),
        ];
        let clusters = cluster_failures(&runs);
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].count, 2);
        assert_eq!(clusters[0].signature, "E0308");
    }

    #[test]
    fn diff_marks_a_new_failure_as_a_regression() {
        let mut baseline = Trajectory::new("todo-api", "todo", "gpt-4", "v1", "v1");
        baseline.outcome = "success".into();
        let mut other = Trajectory::new("todo-api", "todo", "gpt-4o", "v1", "v1");
        other.outcome = "failure".into();
        other.failure_modes = vec!["spec_mismatch".into()];
        let diffs = diff_against_baseline(&[baseline.clone(), other], &baseline);
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].kind, "regression");
    }
}
