//! Build check, runtime smoke test, and endpoint checks for a generated service.
//!
//! The backend agent calls this after it writes `src/main.rs`. Replay uses the
//! same functions, so a benchmark server is held to the same bar as an
//! Actix-web project produced interactively.

use serde::{Deserialize, Serialize};
use std::fs;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BuildReport {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SmokeReport {
    pub success: bool,
    pub detail: String,
    pub startup_ms: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EndpointCheck {
    pub route: String,
    pub url: String,
    pub expected_status: u16,
    pub actual_status: Option<u16>,
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProbeReport {
    pub smoke: SmokeReport,
    pub endpoints: Vec<EndpointCheck>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VerificationReport {
    pub build: BuildReport,
    pub smoke: Option<SmokeReport>,
    pub endpoints: Vec<EndpointCheck>,
}

pub struct RunningServer {
    child: Child,
    pub port: u16,
}

impl RunningServer {
    pub fn stop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Drop for RunningServer {
    fn drop(&mut self) {
        self.stop();
    }
}

#[tracing::instrument(
    name = "tool.call",
    skip(project_dir),
    fields(tool = "cargo_build", agent = "Backend Developer")
)]
pub async fn build_project(
    project_dir: &Path,
) -> Result<BuildReport, Box<dyn std::error::Error + Send + Sync>> {
    let dir = project_dir.to_path_buf();
    let output = tokio::task::spawn_blocking(move || cargo_build_blocking(&dir))
        .await
        .map_err(|err| format!("build task failed: {err}"))??;
    Ok(output)
}

fn absolute_dir(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(path))
            .unwrap_or_else(|_| path.to_path_buf())
    }
}

fn cargo_build_blocking(
    project_dir: &Path,
) -> Result<BuildReport, Box<dyn std::error::Error + Send + Sync>> {
    let project_dir = absolute_dir(project_dir);
    let output = Command::new("cargo")
        .arg("build")
        .arg("--color")
        .arg("never")
        .current_dir(&project_dir)
        // Keep the binary under this project even when it is a workspace member.
        // The path must be absolute: cargo resolves a relative CARGO_TARGET_DIR
        // from the project directory, which would nest target/ one level too deep.
        .env("CARGO_TARGET_DIR", project_dir.join("target"))
        .env("CARGO_TERM_COLOR", "never")
        .env_remove("RUSTFLAGS")
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()?;
    Ok(BuildReport {
        success: output.status.success(),
        stdout: tail(&String::from_utf8_lossy(&output.stdout), 4000),
        stderr: tail(&String::from_utf8_lossy(&output.stderr), 8000),
    })
}

pub fn package_name(project_dir: &Path) -> Option<String> {
    let text = fs::read_to_string(project_dir.join("Cargo.toml")).ok()?;
    let mut in_package = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_package = trimmed == "[package]";
            continue;
        }
        if !in_package {
            continue;
        }
        let Some((key, value)) = trimmed.split_once('=') else {
            continue;
        };
        if key.trim() != "name" {
            continue;
        }
        let name = value.trim().trim_matches('"').trim();
        if !name.is_empty() {
            return Some(name.to_string());
        }
    }
    None
}

pub fn binary_path(project_dir: &Path) -> PathBuf {
    let project_dir = absolute_dir(project_dir);
    let name = package_name(&project_dir).unwrap_or_else(|| "generated_service".to_string());
    project_dir.join("target").join("debug").join(name)
}

pub fn free_port() -> Result<u16, Box<dyn std::error::Error + Send + Sync>> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    drop(listener);
    Ok(port)
}

pub fn spawn_binary(
    project_dir: &Path,
    port: u16,
) -> Result<RunningServer, Box<dyn std::error::Error + Send + Sync>> {
    let binary = binary_path(project_dir);
    if !binary.exists() {
        return Err(format!("built binary not found at {}", binary.display()).into());
    }
    let child = Command::new(&binary)
        .current_dir(project_dir)
        .env("PORT", port.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    Ok(RunningServer { child, port })
}

#[tracing::instrument(
    name = "tool.call",
    skip(project_dir, routes),
    fields(tool = "runtime_smoke", agent = "Backend Developer")
)]
pub async fn probe(
    project_dir: &Path,
    routes: &[String],
) -> Result<ProbeReport, Box<dyn std::error::Error + Send + Sync>> {
    let port = free_port()?;
    let mut server = spawn_binary(project_dir, port)?;
    let smoke = wait_for_http(server.port, smoke_timeout()).await;
    let endpoints = if smoke.success {
        check_endpoints(server.port, routes).await
    } else {
        routes
            .iter()
            .map(|route| EndpointCheck {
                route: route.clone(),
                url: endpoint_url(port, route),
                expected_status: 200,
                actual_status: None,
                success: false,
                error: Some("server did not answer the smoke request".to_string()),
            })
            .collect()
    };
    server.stop();
    Ok(ProbeReport { smoke, endpoints })
}

pub async fn wait_for_http(port: u16, timeout: Duration) -> SmokeReport {
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_millis(800))
        .build()
    {
        Ok(client) => client,
        Err(err) => {
            return SmokeReport {
                success: false,
                detail: format!("http client: {err}"),
                startup_ms: 0,
            };
        }
    };
    let url = format!("http://127.0.0.1:{port}/health");
    let started = Instant::now();
    let deadline = started + timeout;
    loop {
        match client.get(&url).send().await {
            Ok(response) => {
                return SmokeReport {
                    success: true,
                    detail: format!("GET /health -> {}", response.status()),
                    startup_ms: started.elapsed().as_millis(),
                };
            }
            Err(err) => {
                if Instant::now() >= deadline {
                    return SmokeReport {
                        success: false,
                        detail: format!("no HTTP response from {url}: {err}"),
                        startup_ms: started.elapsed().as_millis(),
                    };
                }
                tokio::time::sleep(Duration::from_millis(40)).await;
            }
        }
    }
}

pub async fn check_endpoints(port: u16, routes: &[String]) -> Vec<EndpointCheck> {
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(endpoint_timeout_secs()))
        .build()
    {
        Ok(client) => client,
        Err(err) => {
            return routes
                .iter()
                .map(|route| EndpointCheck {
                    route: route.clone(),
                    url: endpoint_url(port, route),
                    expected_status: 200,
                    actual_status: None,
                    success: false,
                    error: Some(err.to_string()),
                })
                .collect();
        }
    };

    let mut checks = Vec::with_capacity(routes.len());
    for route in routes {
        let url = endpoint_url(port, route);
        let span = tracing::info_span!(
            "tool.call",
            tool = "endpoint_test",
            agent = "Backend Developer",
            route = %route
        );
        let check = {
            let _entered = span.enter();
            match client.get(&url).send().await {
                Ok(response) => {
                    let status = response.status().as_u16();
                    EndpointCheck {
                        route: route.clone(),
                        url,
                        expected_status: 200,
                        actual_status: Some(status),
                        success: status == 200,
                        error: None,
                    }
                }
                Err(err) => EndpointCheck {
                    route: route.clone(),
                    url,
                    expected_status: 200,
                    actual_status: None,
                    success: false,
                    error: Some(err.to_string()),
                },
            }
        };
        checks.push(check);
    }
    checks
}

pub fn endpoint_url(port: u16, route: &str) -> String {
    if route.starts_with('/') {
        format!("http://127.0.0.1:{port}{route}")
    } else {
        format!("http://127.0.0.1:{port}/{route}")
    }
}

pub fn static_get_routes(method: &str, is_dynamic: &str) -> bool {
    method == "get" && is_dynamic == "false"
}

fn smoke_timeout() -> Duration {
    let seconds = std::env::var("SMOKE_TIMEOUT_SECS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(15);
    Duration::from_secs(seconds)
}

fn endpoint_timeout_secs() -> u64 {
    std::env::var("ENDPOINT_TIMEOUT_SECS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(5)
}

pub fn tail(text: &str, max_chars: usize) -> String {
    let count = text.chars().count();
    if count <= max_chars {
        text.to_string()
    } else {
        text.chars().skip(count - max_chars).collect()
    }
}

pub fn rustc_error_code(stderr: &str) -> Option<String> {
    let marker = "error[E";
    let start = stderr.find(marker)?;
    let digits: String = stderr[start + marker.len()..]
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .collect();
    if digits.is_empty() {
        None
    } else {
        Some(format!("E{digits}"))
    }
}

#[cfg(test)]
fn write_project(dir: &Path, name: &str, main_rs: &str) {
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(
        dir.join("Cargo.toml"),
        format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"),
    )
    .unwrap();
    fs::write(dir.join("src/main.rs"), main_rs).unwrap();
}

#[cfg(test)]
const OK_SERVER: &str = r#"
use std::io::{Read, Write};
use std::net::TcpListener;

fn main() {
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let listener = TcpListener::bind(format!("127.0.0.1:{port}")).expect("bind");
    for conn in listener.incoming() {
        let Ok(mut stream) = conn else { continue };
        let mut buf = [0_u8; 2048];
        let n = stream.read(&mut buf).unwrap_or(0);
        let req = String::from_utf8_lossy(&buf[..n]);
        let path = req.split_whitespace().nth(1).unwrap_or("/");
        let path = path.split('?').next().unwrap_or("/");
        let (status, body) = match path {
            "/health" => ("200 OK", "{\"status\":\"ok\"}"),
            "/todos" => ("200 OK", "[{\"id\":1,\"title\":\"sample\"}]"),
            _ => ("404 NOT FOUND", "{\"error\":\"not found\"}"),
        };
        let len = body.len();
        let resp = format!(
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {len}\r\nConnection: close\r\n\r\n{body}"
        );
        let _ = stream.write_all(resp.as_bytes());
    }
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_package_name_and_rustc_code() {
        let dir = std::env::temp_dir().join(format!("autogpt-pkg-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        write_project(&dir, "probe_name", "fn main() {}");
        assert_eq!(package_name(&dir).as_deref(), Some("probe_name"));
        let _ = fs::remove_dir_all(&dir);
        assert_eq!(
            rustc_error_code("error[E0308]: mismatched types\n"),
            Some("E0308".to_string())
        );
        assert!(static_get_routes("get", "false"));
        assert!(!static_get_routes("GET", "false"));
        assert!(!static_get_routes("get", "true"));
    }

    #[tokio::test]
    async fn verifies_a_service_that_builds_and_serves() {
        let dir = std::env::temp_dir().join(format!(
            "autogpt-ok-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&dir);
        write_project(&dir, "probe_ok", OK_SERVER);
        let build = build_project(&dir).await.expect("build");
        assert!(build.success, "{}", build.stderr);
        let probe = probe(&dir, &["/health".into(), "/todos".into()])
            .await
            .expect("probe");
        assert!(probe.smoke.success, "{}", probe.smoke.detail);
        assert!(
            probe.endpoints.iter().all(|check| check.success),
            "{probe:?}"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn build_check_reports_compiler_errors() {
        let dir = std::env::temp_dir().join(format!(
            "autogpt-bad-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&dir);
        write_project(
            &dir,
            "probe_bad",
            "fn main() { let _port: i32 = \"8080\"; }\n",
        );
        let build = build_project(&dir).await.expect("build command");
        assert!(!build.success);
        assert_eq!(rustc_error_code(&build.stderr).as_deref(), Some("E0308"));
        let _ = fs::remove_dir_all(&dir);
    }
}
