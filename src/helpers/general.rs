use crate::apis::call_request::call_gpt;
use crate::helpers::command_line::PrintCommand;
use crate::models::agents::agent_traits::AgentError;
use crate::models::general::llm::Message;
use crate::run_context;
use crate::trajectory;
use reqwest::Client;
use serde::de::DeserializeOwned;
use std::fs;
use std::path::PathBuf;
use std::time::Instant;
use tracing::Instrument;

pub fn web_server_project_path() -> PathBuf {
    if let Some(dir) = run_context::project_dir() {
        return dir;
    }
    std::env::var("WEB_SERVER_PROJECT_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("web_template"))
}

pub fn code_template_path() -> PathBuf {
    if run_context::project_dir().is_some() {
        return web_server_project_path()
            .join("src")
            .join("code_template.rs");
    }
    std::env::var("CODE_TEMPLATE_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            web_server_project_path()
                .join("src")
                .join("code_template.rs")
        })
}

pub fn exec_main_path() -> PathBuf {
    if run_context::project_dir().is_none()
        && let Ok(path) = std::env::var("EXEC_MAIN_PATH")
    {
        return PathBuf::from(path);
    }
    web_server_project_path().join("src").join("main.rs")
}

pub fn api_schema_path() -> PathBuf {
    if let Some(dir) = run_context::project_dir() {
        return dir.join("api_schema.json");
    }
    std::env::var("API_SCHEMA_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("schemas/api_schema.json"))
}

// Extend ai function to encourage specific output.
// Prompt v1 is the original instruction. Prompt v2 and tool schema v2 are
// explicit versions the replay harness swaps in.
pub fn extend_ai_function(ai_func: fn(&str) -> &'static str, func_input: &str) -> Message {
    let ai_function_str: &str = ai_func(func_input);
    let prompt_version = run_context::prompt_version();
    let mut msg = if prompt_version == "v2" {
        format!(
            "FUNCTION: {ai_function_str}\n\
             INSTRUCTION: You are a function printer for prompt schema v2.\n\
             Return only the function result for this input: {func_input}.\n\
             No markdown fences, no commentary, no leading label.\n\
             If the result is JSON, emit raw JSON."
        )
    } else {
        format!(
            "FUNCTION: {}
  INSTRUCTION: You are a function printer. You ONLY print the results of functions.
  Nothing else. No commentary. Here is the input to the function: {}.
  Print out what the function will return.",
            ai_function_str, func_input
        )
    };

    if run_context::tool_schema_version() == "v2" {
        msg.push_str(
            "\nTOOL_SCHEMA v2: JSON booleans are true or false. Route method is lowercase \
             \"get\" or \"post\". is_route_dynamic is the string \"true\" or \"false\". \
             Do not wrap the result in code fences.",
        );
    }

    Message {
        role: "system".to_string(),
        content: msg,
    }
}

// Performs call to LLM GPT.
pub async fn ai_task_request(
    msg_context: String,
    agent_position: &str,
    agent_operation: &str,
    function_pass: for<'a> fn(&'a str) -> &'static str,
) -> Result<String, AgentError> {
    let extended_msg: Message = extend_ai_function(function_pass, &msg_context);
    PrintCommand::AICall.print_agent_message(agent_position, agent_operation);

    let started = Instant::now();
    let span = tracing::info_span!(
        "tool.call",
        agent = %agent_position,
        tool = %agent_operation,
        otel.kind = "client"
    );
    let extended_for_retry = extended_msg.clone();
    let result = async move {
        match call_gpt(vec![extended_msg]).await {
            Ok(response) => Ok(response),
            Err(first) => match call_gpt(vec![extended_for_retry]).await {
                Ok(response) => Ok(response),
                Err(second) => Err(format!("OpenAI call failed twice: {first}; {second}")),
            },
        }
    }
    .instrument(span)
    .await;

    let elapsed = started.elapsed().as_millis();
    match result {
        Ok(response) => {
            trajectory::record_tool(
                agent_position,
                agent_operation,
                &msg_context,
                &response,
                true,
                None,
                elapsed,
            );
            Ok(response)
        }
        Err(err) => {
            trajectory::record_tool(
                agent_position,
                agent_operation,
                &msg_context,
                "",
                false,
                Some(err.clone()),
                elapsed,
            );
            Err(err.into())
        }
    }
}

// Performs call to LLM GPT and decodes a JSON body.
pub async fn ai_task_request_decoded<T: DeserializeOwned>(
    msg_context: String,
    agent_position: &str,
    agent_operation: &str,
    function_pass: for<'a> fn(&'a str) -> &'static str,
) -> Result<T, AgentError> {
    let llm_response: String =
        ai_task_request(msg_context, agent_position, agent_operation, function_pass).await?;
    match serde_json::from_str::<T>(llm_response.as_str()) {
        Ok(decoded) => Ok(decoded),
        Err(err) => {
            let message = format!("Failed to decode ai response: {err}; body: {llm_response}");
            trajectory::mark_last_step_failed(&message);
            Err(message.into())
        }
    }
}

// Check whether request url is valid.
pub async fn check_status_code(client: &Client, url: &str) -> Result<u16, reqwest::Error> {
    let response = client.get(url).send().await?;
    Ok(response.status().as_u16())
}

pub fn read_code_template_contents() -> Result<String, AgentError> {
    fs::read_to_string(code_template_path()).map_err(|err| {
        format!(
            "Failed to read code template at {}: {err}",
            code_template_path().display()
        )
        .into()
    })
}

pub fn read_exec_main_contents() -> Result<String, AgentError> {
    fs::read_to_string(exec_main_path()).map_err(|err| {
        format!(
            "Failed to read generated main at {}: {err}",
            exec_main_path().display()
        )
        .into()
    })
}

pub fn save_backend_code(contents: &str) -> Result<(), AgentError> {
    let path = exec_main_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, contents)?;
    Ok(())
}

pub fn save_api_endpoints(api_endpoints: &str) -> Result<(), AgentError> {
    let path = api_schema_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, api_endpoints)?;
    Ok(())
}

// Keep the URLs whose checks did not fail. Same filter the architect used inline.
pub fn urls_to_keep(urls: &[String], failed: &[String]) -> Vec<String> {
    urls.iter()
        .filter(|url| !failed.contains(url))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai_functions::aifunc_managing::convert_user_input_to_goal;
    use crate::run_context::{RunConfig, scope};

    #[test]
    fn tests_extending_ai_function() {
        let extended_msg = extend_ai_function(convert_user_input_to_goal, "dummy variable");
        assert_eq!(extended_msg.role, "system");
        assert!(extended_msg.content.contains("FUNCTION:"));
        assert!(
            extended_msg
                .content
                .contains("fn convert_user_input_to_goal")
        );
        assert!(extended_msg.content.contains("dummy variable"));
        assert!(!extended_msg.content.contains("TOOL_SCHEMA v2"));
    }

    #[tokio::test]
    async fn prompt_and_tool_schema_versions_change_the_prompt() {
        let baseline = extend_ai_function(convert_user_input_to_goal, "input");
        let prompt_v2 = scope(
            RunConfig {
                prompt_version: "v2".to_string(),
                ..RunConfig::default()
            },
            async { extend_ai_function(convert_user_input_to_goal, "input") },
        )
        .await;
        let schema_v2 = scope(
            RunConfig {
                tool_schema_version: "v2".to_string(),
                ..RunConfig::default()
            },
            async { extend_ai_function(convert_user_input_to_goal, "input") },
        )
        .await;
        assert!(prompt_v2.content.contains("prompt schema v2"));
        assert_ne!(baseline.content, prompt_v2.content);
        assert!(schema_v2.content.contains("TOOL_SCHEMA v2"));
        assert!(schema_v2.content.contains("FUNCTION:"));
    }

    #[test]
    fn keeps_urls_that_were_not_failed() {
        let urls = vec!["https://ok.example".into(), "https://bad.example".into()];
        let failed = vec!["https://bad.example".into()];
        assert_eq!(
            urls_to_keep(&urls, &failed),
            vec!["https://ok.example".to_string()]
        );
    }

    #[tokio::test]
    #[ignore = "calls OpenAI; set OPEN_AI_KEY and run with --ignored"]
    async fn tests_ai_task_request() {
        let ai_func_param = "Build me a webserver for making stock price api requests.".to_string();
        let res = ai_task_request(
            ai_func_param,
            "Managing Agent",
            get_function_string!(convert_user_input_to_goal),
            convert_user_input_to_goal,
        )
        .await
        .expect("live llm call");
        assert!(res.len() > 20);
    }
}
