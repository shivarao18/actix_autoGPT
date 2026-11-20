use crate::helpers::command_line::get_user_response;
use crate::models::agents_manager::managing_agent::ManagingAgent;
use crate::replay::{self, ReplayOptions};
use crate::run_context::{self, RunConfig};
use crate::verification;
use std::path::PathBuf;
use tracing::Instrument;

/// Trace file requested by `trace-demo`, if that command is the one being run.
pub fn traces_file_override(args: &[String]) -> Option<PathBuf> {
    if args.first().map(String::as_str) == Some("trace-demo") {
        Some(PathBuf::from(
            flag(args, "--out").unwrap_or_else(|| "docs/sample_trace.json".to_string()),
        ))
    } else {
        None
    }
}

pub async fn dispatch(args: &[String]) -> Result<i32, Box<dyn std::error::Error + Send + Sync>> {
    match args.first().map(String::as_str) {
        None | Some("run") => interactive().await,
        Some("verify") => verify(args).await,
        Some("replay") => replay_cmd(args, None, true).await,
        Some("trace-demo") => replay_cmd(args, Some("todo-api".to_string()), false).await,
        Some("eval") => eval_cmd(args).await,
        Some("help") | Some("--help") | Some("-h") => {
            print_help();
            Ok(0)
        }
        Some(other) => {
            eprintln!("unknown command '{other}'");
            print_help();
            Ok(2)
        }
    }
}

fn print_help() {
    println!(
        "\
actix_autogpt — multi-agent Actix service builder

  cargo run --                              interactive build
  cargo run -- verify --project web_template --route /health
  cargo run -- replay --out docs/sample_replay_report.md --trajectories docs/sample_trajectories.json
  cargo run -- trace-demo --out docs/sample_trace.json
  cargo run -- eval --trajectories docs/sample_trajectories.json --out docs/sample_eval_scores.json

Environment is documented in README.md and .env.example.
Live model calls need OPEN_AI_KEY. Replay, trace-demo, and eval use fixtures."
    );
}

async fn interactive() -> Result<i32, Box<dyn std::error::Error + Send + Sync>> {
    let request = get_user_response("What website are we building today?");
    let cfg = RunConfig::default();
    run_context::scope(cfg, async move {
        let span = tracing::info_span!("agent.run", agent = "Project Manager");
        async move {
            let mut manager = ManagingAgent::new(request).await?;
            manager.execute_project().await?;
            println!("\nGoal: {}", manager.factsheet().project_description);
            Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
        }
        .instrument(span)
        .await
    })
    .await?;
    Ok(0)
}

async fn verify(args: &[String]) -> Result<i32, Box<dyn std::error::Error + Send + Sync>> {
    let project = flag(args, "--project").unwrap_or_else(|| "web_template".to_string());
    let mut routes = flags(args, "--route");
    if routes.is_empty() {
        routes.push("/health".to_string());
    }
    let project = PathBuf::from(project);
    let build = verification::build_project(&project).await?;
    println!("build: {}", if build.success { "ok" } else { "failed" });
    if !build.success {
        eprintln!("{}", build.stderr);
        return Ok(1);
    }
    let probe = verification::probe(&project, &routes).await?;
    println!(
        "smoke: {} ({})",
        if probe.smoke.success { "ok" } else { "failed" },
        probe.smoke.detail
    );
    let mut failed = !probe.smoke.success;
    for check in &probe.endpoints {
        println!(
            "endpoint {} -> {:?} {}",
            check.route,
            check.actual_status,
            if check.success { "ok" } else { "failed" }
        );
        failed |= !check.success;
    }
    Ok(if failed { 1 } else { 0 })
}

async fn replay_cmd(
    args: &[String],
    default_spec: Option<String>,
    use_out_as_report: bool,
) -> Result<i32, Box<dyn std::error::Error + Send + Sync>> {
    let benchmarks = flag(args, "--benchmarks")
        .map(PathBuf::from)
        .unwrap_or_else(replay::benchmarks_dir);
    let spec_filter = flag(args, "--spec").or(default_spec);
    // trace-demo uses --out for the JSON trace file, not the markdown report.
    let report_path = if use_out_as_report {
        flag(args, "--out").map(PathBuf::from)
    } else {
        None
    };
    let trajectories_path = flag(args, "--trajectories").map(PathBuf::from);
    let (runs, _diffs, clusters) = replay::run(&ReplayOptions {
        benchmarks,
        spec_filter,
        report_path,
        trajectories_path,
        echo_report: use_out_as_report && flag(args, "--out").is_none(),
    })
    .await?;
    let failures = runs.iter().filter(|run| run.outcome != "success").count();
    println!(
        "replay finished: {} runs, {failures} failures, {} clusters",
        runs.len(),
        clusters.len()
    );
    Ok(0)
}

async fn eval_cmd(args: &[String]) -> Result<i32, Box<dyn std::error::Error + Send + Sync>> {
    let path = flag(args, "--trajectories").ok_or("eval requires --trajectories FILE")?;
    let raw = std::fs::read_to_string(&path)?;
    let runs: Vec<crate::trajectory::Trajectory> = serde_json::from_str(&raw)?;
    let scored = replay::rescore_saved(runs)?;
    let body = serde_json::to_string_pretty(&scored)?;
    if let Some(out) = flag(args, "--out") {
        if let Some(parent) = PathBuf::from(&out).parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&out, &body)?;
        println!("wrote {out}");
    } else {
        println!("{body}");
    }
    let failures = scored.iter().filter(|run| run.outcome != "success").count();
    println!(
        "deterministic eval: {} runs, {failures} failures",
        scored.len()
    );
    Ok(0)
}

fn flag(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|arg| arg == name)
        .and_then(|index| args.get(index + 1).cloned())
}

fn flags(args: &[String], name: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut index = 0;
    while index < args.len() {
        if args[index] == name
            && let Some(value) = args.get(index + 1)
        {
            values.push(value.clone());
            index += 2;
            continue;
        }
        index += 1;
    }
    values
}
