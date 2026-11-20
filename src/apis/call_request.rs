use crate::llm;
use crate::models::general::llm::Message;

// Call Large Language Model (i.e. GPT-4). Replay installs a fixture model on
// the task-local client; otherwise this is the OpenAI chat completions API.
pub async fn call_gpt(
    messages: Vec<Message>,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    llm::complete(messages).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "calls OpenAI; set OPEN_AI_KEY and run with --ignored"]
    async fn tests_call_to_openai() {
        let message = Message {
            role: "user".to_string(),
            content: "Hi there, this is a test. Give me a short reponse.".to_string(),
        };
        let res = call_gpt(vec![message]).await.expect("openai call");
        assert!(!res.is_empty());
    }
}
