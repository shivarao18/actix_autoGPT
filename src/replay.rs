//! Re-run benchmark specs across model, prompt, and tool-schema versions.
//!
//! The agents are the same types the interactive binary uses. A fixture model
//! stands in for the LLM, then the shared verifier builds and probes the
//! service each run produced.

use crate::llm::{self, LanguageModel, MockModel};
use crate::models::agents_manager::managing_agent::ManagingAgent;
use crate::run_context::{self, RunConfig};
use crate::scoring::{self, ExpectedSpec};
use crate::trajectory::{
    self, FailureCluster, OutcomeDiff, Trajectory, cluster_failures, diff_against_baseline,
    render_report,
};
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tracing::Instrument;

#[derive(Debug, Clone, Deserialize)]
pub struct Version {
    pub model: String,
    pub prompt_version: String,
    pub tool_schema_version: String,
}

#[derive(Debug, Clone)]
pub struct ReplayOptions {
    pub benchmarks: PathBuf,
    pub spec_filter: Option<String>,
    pub report_path: Option<PathBuf>,
    pub trajectories_path: Option<PathBuf>,
    /// When no report path is set, print the markdown report.
    pub echo_report: bool,
}

#[derive(Debug, Deserialize)]
struct SpecFile {
    specs: Vec<ExpectedSpec>,
}

#[derive(Debug, Deserialize)]
struct VersionFile {
    versions: Vec<Version>,
}

pub fn benchmarks_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("benchmarks")
}

pub fn load_specs(
    benchmarks: &Path,
) -> Result<Vec<ExpectedSpec>, Box<dyn std::error::Error + Send + Sync>> {
    let path = benchmarks.join("specs.json");
    let file: SpecFile = serde_json::from_str(&fs::read_to_string(&path)?)?;
    Ok(file.specs)
}

pub fn load_versions(
    benchmarks: &Path,
) -> Result<Vec<Version>, Box<dyn std::error::Error + Send + Sync>> {
    let path = benchmarks.join("matrix.json");
    let file: VersionFile = serde_json::from_str(&fs::read_to_string(&path)?)?;
    Ok(file.versions)
}

pub async fn run_one(
    benchmarks: &Path,
    spec: &ExpectedSpec,
    version: &Version,
) -> Result<Trajectory, Box<dyn std::error::Error + Send + Sync>> {
    let fixture = resolve_fixture(benchmarks, version, &spec.id)?;
    let mock: Arc<dyn LanguageModel> = Arc::new(MockModel::from_fixture(&fixture, benchmarks)?);
    let temp = unique_dir(&spec.id, version);
    if temp.exists() {
        fs::remove_dir_all(&temp)?;
    }
    copy_dir(&benchmarks.join("skeleton"), &temp)?;

    let record = Arc::new(Mutex::new(Trajectory::new(
        spec.id.clone(),
        spec.request.clone(),
        version.model.clone(),
        version.prompt_version.clone(),
        version.tool_schema_version.clone(),
    )));
    let cfg = RunConfig::replay(
        version.model.clone(),
        version.prompt_version.clone(),
        version.tool_schema_version.clone(),
        temp.clone(),
    );

    let recorded = record.clone();
    let request = spec.request.clone();
    let spec_id = spec.id.clone();
    let result = run_context::scope(
        cfg,
        llm::scope_model(
            mock,
            trajectory::scope(recorded, async move {
                let span = tracing::info_span!(
                    "agent.run",
                    agent = "Project Manager",
                    spec_id = %spec_id
                );
                async move {
                    match ManagingAgent::new(request).await {
                        Ok(mut manager) => {
                            let exec_err = manager
                                .execute_project()
                                .await
                                .err()
                                .map(|err| err.to_string());
                            Ok::<_, Box<dyn std::error::Error + Send + Sync>>((
                                Some(manager.factsheet().clone()),
                                exec_err,
                            ))
                        }
                        Err(err) => Ok((None, Some(err.to_string()))),
                    }
                }
                .instrument(span)
                .await
            }),
        ),
    )
    .await?;

    {
        let mut guard = record.lock().expect("trajectory lock");
        guard.factsheet = result.0;
        guard.agent_error = result.1;
    }
    let snapshot = record.lock().expect("trajectory lock").clone();
    let finalized = scoring::finalize(snapshot, spec);
    if !crate::run_context::env_flag("KEEP_REPLAY_DIRS") {
        let _ = fs::remove_dir_all(&temp);
    }
    Ok(finalized)
}

pub async fn run(
    options: &ReplayOptions,
) -> Result<
    (Vec<Trajectory>, Vec<OutcomeDiff>, Vec<FailureCluster>),
    Box<dyn std::error::Error + Send + Sync>,
> {
    let mut specs = load_specs(&options.benchmarks)?;
    if let Some(filter) = &options.spec_filter {
        specs.retain(|spec| &spec.id == filter);
        if specs.is_empty() {
            return Err(format!("no benchmark spec matched {filter}").into());
        }
    }
    let versions = load_versions(&options.benchmarks)?;
    let mut runs = Vec::new();
    for spec in &specs {
        for version in &versions {
            println!(
                "replay {} on {} / prompt {} / schema {}",
                spec.id, version.model, version.prompt_version, version.tool_schema_version
            );
            runs.push(run_one(&options.benchmarks, spec, version).await?);
        }
    }

    let mut diffs = Vec::new();
    for spec in &specs {
        if let Some(baseline) = runs.iter().find(|run| {
            run.spec_id == spec.id
                && run.model == "gpt-4"
                && run.prompt_version == "v1"
                && run.tool_schema_version == "v1"
        }) {
            diffs.extend(diff_against_baseline(&runs, baseline));
        }
    }
    let clusters = cluster_failures(&runs);
    let report = render_report(&runs, &diffs, &clusters);
    if let Some(path) = &options.report_path {
        write_text(path, &report)?;
        println!("wrote {}", path.display());
    } else if options.echo_report {
        println!("{report}");
    }
    if let Some(path) = &options.trajectories_path {
        write_text(path, &serde_json::to_string_pretty(&runs)?)?;
        println!("wrote {}", path.display());
    }
    Ok((runs, diffs, clusters))
}

pub fn resolve_fixture(
    benchmarks: &Path,
    version: &Version,
    spec_id: &str,
) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    let specific = benchmarks
        .join("fixtures")
        .join(&version.model)
        .join(&version.prompt_version)
        .join(&version.tool_schema_version)
        .join(format!("{spec_id}.json"));
    if specific.exists() {
        return Ok(specific);
    }
    let fallback = benchmarks
        .join("fixtures")
        .join("gpt-4")
        .join("v1")
        .join("v1")
        .join(format!("{spec_id}.json"));
    if fallback.exists() {
        return Ok(fallback);
    }
    Err(format!(
        "no fixture for {spec_id} at {} or {}",
        specific.display(),
        fallback.display()
    )
    .into())
}

fn unique_dir(spec_id: &str, version: &Version) -> PathBuf {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let slug = format!(
        "autogpt-{}-{}-{}-{}-{}-{}",
        spec_id,
        version.model,
        version.prompt_version,
        version.tool_schema_version,
        std::process::id(),
        stamp
    )
    .replace('/', "-");
    std::env::temp_dir().join(slug)
}

fn copy_dir(src: &Path, dst: &Path) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        if entry.file_name() == "target" {
            continue;
        }
        let target = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

fn write_text(path: &Path, body: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, body)?;
    Ok(())
}

pub fn rescore_saved(
    runs: Vec<Trajectory>,
) -> Result<Vec<Trajectory>, Box<dyn std::error::Error + Send + Sync>> {
    let mut scored = Vec::with_capacity(runs.len());
    for run in runs {
        let scope = run
            .expected_scope
            .ok_or_else(|| format!("{} is missing expected_scope", run.spec_id))?;
        let spec = ExpectedSpec {
            id: run.spec_id.clone(),
            request: run.spec.clone(),
            expected_goal_contains: run.expected_goal_contains.clone(),
            expected_scope: scope,
            expected_routes: run.expected_routes.clone(),
            expected_tools: run.expected_tools.clone(),
        };
        scored.push(scoring::finalize(run, &spec));
    }
    Ok(scored)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback_fixture_resolves_for_an_unspecified_version() {
        let benchmarks = benchmarks_dir();
        let version = Version {
            model: "gpt-4o".into(),
            prompt_version: "v1".into(),
            tool_schema_version: "v1".into(),
        };
        let path = resolve_fixture(&benchmarks, &version, "notes-api").expect("fixture");
        assert!(path.ends_with("gpt-4/v1/v1/notes-api.json") || path.ends_with("notes-api.json"));
        assert!(path.exists());
    }

    #[tokio::test]
    async fn replays_one_benchmark_spec() {
        let benchmarks = benchmarks_dir();
        let spec = load_specs(&benchmarks)
            .expect("specs")
            .into_iter()
            .find(|spec| spec.id == "todo-api")
            .expect("todo-api");
        let version = Version {
            model: "gpt-4".into(),
            prompt_version: "v1".into(),
            tool_schema_version: "v1".into(),
        };
        let trajectory = run_one(&benchmarks, &spec, &version).await.expect("replay");
        assert_eq!(trajectory.outcome, "success", "{trajectory:?}");
        assert!(
            trajectory
                .steps
                .iter()
                .any(|step| step.tool == "runtime_smoke" && step.success)
        );
        assert!(trajectory.deterministic.unwrap().spec_compliance >= 0.99);
    }
}
