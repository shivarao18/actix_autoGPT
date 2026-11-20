//! Per-run settings that the existing agents read without new parameters.
//!
//! Interactive runs use the defaults (prompt v1, tool schema v1, gpt-4).
//! Replay nests a [`RunConfig`] in a Tokio task-local so model, prompt, and
//! tool-schema versions change the prompt the agents already build.

use std::env;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

tokio::task_local! {
    static RUN: RunConfig;
}

#[derive(Clone)]
pub struct RunConfig {
    pub model_name: String,
    pub prompt_version: String,
    pub tool_schema_version: String,
    pub auto_confirm: bool,
    pub skip_network_checks: bool,
    pub project_dir: Option<PathBuf>,
    pub(crate) phase: Arc<Mutex<String>>,
}

impl Default for RunConfig {
    fn default() -> Self {
        Self {
            model_name: env_or("OPENAI_MODEL", "gpt-4"),
            prompt_version: env_or("PROMPT_VERSION", "v1"),
            tool_schema_version: env_or("TOOL_SCHEMA_VERSION", "v1"),
            auto_confirm: env_flag("AUTO_CONFIRM"),
            skip_network_checks: env_flag("SKIP_NETWORK_CHECKS"),
            project_dir: None,
            phase: Arc::new(Mutex::new("Discovery".to_string())),
        }
    }
}

impl RunConfig {
    pub fn replay(
        model_name: impl Into<String>,
        prompt_version: impl Into<String>,
        tool_schema_version: impl Into<String>,
        project_dir: PathBuf,
    ) -> Self {
        Self {
            model_name: model_name.into(),
            prompt_version: prompt_version.into(),
            tool_schema_version: tool_schema_version.into(),
            auto_confirm: true,
            skip_network_checks: true,
            project_dir: Some(project_dir),
            phase: Arc::new(Mutex::new("Discovery".to_string())),
        }
    }

    pub fn set_phase(&self, phase: &str) {
        *self.phase.lock().expect("phase lock") = phase.to_string();
    }

    pub fn phase(&self) -> String {
        self.phase.lock().expect("phase lock").clone()
    }
}

pub async fn scope<F>(config: RunConfig, future: F) -> F::Output
where
    F: std::future::Future,
{
    RUN.scope(config, future).await
}

pub fn model_name() -> String {
    RUN.try_with(|cfg| cfg.model_name.clone())
        .unwrap_or_else(|_| env_or("OPENAI_MODEL", "gpt-4"))
}

pub fn prompt_version() -> String {
    RUN.try_with(|cfg| cfg.prompt_version.clone())
        .unwrap_or_else(|_| env_or("PROMPT_VERSION", "v1"))
}

pub fn tool_schema_version() -> String {
    RUN.try_with(|cfg| cfg.tool_schema_version.clone())
        .unwrap_or_else(|_| env_or("TOOL_SCHEMA_VERSION", "v1"))
}

pub fn auto_confirm() -> bool {
    RUN.try_with(|cfg| cfg.auto_confirm).unwrap_or(false) || env_flag("AUTO_CONFIRM")
}

pub fn skip_network_checks() -> bool {
    RUN.try_with(|cfg| cfg.skip_network_checks).unwrap_or(false) || env_flag("SKIP_NETWORK_CHECKS")
}

pub fn project_dir() -> Option<PathBuf> {
    RUN.try_with(|cfg| cfg.project_dir.clone()).ok().flatten()
}

pub fn set_phase(phase: &str) {
    let _ = RUN.try_with(|cfg| cfg.set_phase(phase));
}

pub fn phase() -> String {
    RUN.try_with(|cfg| cfg.phase())
        .unwrap_or_else(|_| "n/a".to_string())
}

fn env_or(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_string())
}

pub fn env_flag(key: &str) -> bool {
    matches!(
        env::var(key).ok().as_deref().map(str::to_ascii_lowercase),
        Some(ref value) if matches!(value.as_str(), "1" | "true" | "yes" | "on")
    )
}
