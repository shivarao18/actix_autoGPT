//! Language-model client used by every agent call.
//!
//! The default client is OpenAI Chat Completions. Replay and CI install a
//! [`MockModel`] on a task-local so the same agent code runs from fixtures.

use crate::models::general::llm::{ChatCompletion, Message};
use crate::run_context;
use async_trait::async_trait;
use dotenv::dotenv;
use reqwest::Client;
use reqwest::header::{HeaderMap, HeaderValue};
use serde_json::Value;
use std::collections::{HashMap, VecDeque};
use std::env;
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

tokio::task_local! {
    static MODEL: Arc<dyn LanguageModel>;
}

pub const TOOL_NAMES: &[&str] = &[
    "print_improved_webserver_code",
    "print_backend_webserver_code",
    "print_rest_api_endpoints",
    "convert_user_input_to_goal",
    "print_project_scope",
    "print_fixed_code",
    "print_site_urls",
];

#[async_trait]
pub trait LanguageModel: Send + Sync {
    async fn complete(
        &self,
        messages: Vec<Message>,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>>;
}

pub async fn scope_model<F>(model: Arc<dyn LanguageModel>, future: F) -> F::Output
where
    F: std::future::Future,
{
    MODEL.scope(model, future).await
}

pub async fn complete(
    messages: Vec<Message>,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    if let Ok(model) = MODEL.try_with(|current| current.clone()) {
        return model.complete(messages).await;
    }
    OpenAiModel.complete(messages).await
}

pub fn chat_completion(model: &str, messages: Vec<Message>, temperature: f32) -> ChatCompletion {
    ChatCompletion {
        model: model.to_string(),
        messages,
        temperature,
    }
}

pub struct OpenAiModel;

#[async_trait]
impl LanguageModel for OpenAiModel {
    async fn complete(
        &self,
        messages: Vec<Message>,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        dotenv().ok();
        let api_key = env::var("OPEN_AI_KEY")
            .or_else(|_| env::var("OPENAI_API_KEY"))
            .map_err(|_| "OPEN_AI_KEY or OPENAI_API_KEY not found in environment variables")?;
        let api_org = env::var("OPEN_AI_ORG").unwrap_or_default();

        let mut headers = HeaderMap::new();
        headers.insert(
            "authorization",
            HeaderValue::from_str(&format!("Bearer {api_key}"))?,
        );
        if !api_org.is_empty() {
            headers.insert("OpenAI-Organization", HeaderValue::from_str(&api_org)?);
        }

        let timeout_secs: u64 = env::var("OPENAI_TIMEOUT_SECS")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(120);
        let temperature: f32 = env::var("OPENAI_TEMPERATURE")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(0.1);

        let client = Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(timeout_secs))
            .build()?;

        let body = chat_completion(&run_context::model_name(), messages, temperature);
        let response: crate::models::general::llm::APIResponse = client
            .post("https://api.openai.com/v1/chat/completions")
            .json(&body)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        response
            .choices
            .first()
            .map(|choice| choice.message.content.clone())
            .ok_or_else(|| "OpenAI response did not include a choice".into())
    }
}

/// Fixture model. Responses are queued per AI function name. The function
/// name is read from the prompt the `#[ai_function]` macro embeds.
#[derive(Debug)]
pub struct MockModel {
    queues: Mutex<HashMap<String, VecDeque<String>>>,
}

impl MockModel {
    pub fn from_queues(queues: HashMap<String, VecDeque<String>>) -> Self {
        Self {
            queues: Mutex::new(queues),
        }
    }

    pub fn from_fixture(
        path: &Path,
        file_root: &Path,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let raw = fs::read_to_string(path)?;
        let value: Value = serde_json::from_str(&raw)?;
        let responses = value
            .get("responses")
            .and_then(Value::as_object)
            .ok_or("fixture is missing a responses object")?;

        let mut queues = HashMap::new();
        for (tool, entry) in responses {
            let items = match entry {
                Value::Array(items) => items.clone(),
                other => vec![other.clone()],
            };
            let mut queue = VecDeque::new();
            for item in items {
                queue.push_back(resolve_fixture_value(&item, file_root)?);
            }
            if queue.is_empty() {
                return Err(format!("fixture {tool} has no responses").into());
            }
            queues.insert(tool.clone(), queue);
        }
        Ok(Self::from_queues(queues))
    }
}

fn resolve_fixture_value(
    value: &Value,
    file_root: &Path,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    match value {
        Value::String(text) => Ok(text.clone()),
        Value::Object(map) => {
            if let Some(relative) = map.get("file").and_then(Value::as_str) {
                let path = file_root.join(relative);
                return fs::read_to_string(&path).map_err(|err| {
                    format!("failed to read fixture file {}: {err}", path.display()).into()
                });
            }
            Err("fixture response object needs a file field".into())
        }
        other => Ok(other.to_string()),
    }
}

pub fn detect_tool(prompt: &str) -> Option<&'static str> {
    TOOL_NAMES
        .iter()
        .copied()
        .find(|name| prompt.contains(&format!("fn {name}")))
}

#[async_trait]
impl LanguageModel for MockModel {
    async fn complete(
        &self,
        messages: Vec<Message>,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let prompt = messages
            .iter()
            .map(|message| message.content.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        let tool = detect_tool(&prompt).ok_or_else(|| {
            format!(
                "mock LLM could not identify an AI function in the prompt: {}",
                excerpt(&prompt, 240)
            )
        })?;
        let mut queues = self.queues.lock().expect("mock queue lock");
        let queue = queues
            .get_mut(tool)
            .ok_or_else(|| format!("mock LLM has no responses for {tool}"))?;
        if queue.len() > 1 {
            Ok(queue.pop_front().expect("queue length checked"))
        } else {
            Ok(queue.front().expect("fixture queues are non-empty").clone())
        }
    }
}

pub fn excerpt(text: &str, max_chars: usize) -> String {
    let count = text.chars().count();
    if count <= max_chars {
        return text.to_string();
    }
    let head: String = text.chars().take(max_chars / 2).collect();
    let tail: String = text.chars().skip(count - max_chars / 2).collect();
    format!("{head}\n…\n{tail}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_tool_from_function_source() {
        let prompt = "FUNCTION: pub fn print_project_scope ( _project_description : & str ) { }";
        assert_eq!(detect_tool(prompt), Some("print_project_scope"));
    }

    #[test]
    fn chat_completion_uses_the_requested_model() {
        let body = chat_completion(
            "gpt-4o",
            vec![Message {
                role: "user".into(),
                content: "hi".into(),
            }],
            0.1,
        );
        assert_eq!(body.model, "gpt-4o");
        assert_eq!(body.messages.len(), 1);
    }

    #[tokio::test]
    async fn mock_replays_queued_responses_then_repeats_the_last() {
        let mut queues = HashMap::new();
        queues.insert(
            "print_fixed_code".to_string(),
            VecDeque::from(["first".to_string(), "second".to_string()]),
        );
        let model = MockModel::from_queues(queues);
        let messages = vec![Message {
            role: "system".into(),
            content: "pub fn print_fixed_code(_broken: &str) {}".into(),
        }];
        assert_eq!(model.complete(messages.clone()).await.unwrap(), "first");
        assert_eq!(model.complete(messages.clone()).await.unwrap(), "second");
        assert_eq!(model.complete(messages).await.unwrap(), "second");
    }
}
