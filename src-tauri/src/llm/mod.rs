pub mod claude;
pub mod gemini;
pub mod ollama;
pub mod openai_compat;
pub mod stream;

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::atomic::AtomicBool;

#[derive(Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

pub struct ChatRequest {
    pub provider: String,
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub system_prompt: String,
    pub messages: Vec<ChatMessage>,
    pub temperature: f32,
    pub top_p: f32,
    pub max_tokens: u32,
    pub supports_sampling: bool,
    pub max_tokens_field: String,
}

pub async fn stream_chat<F>(
    http: &Client,
    request: &ChatRequest,
    cancel: &AtomicBool,
    on_token: F,
) -> Result<String, String>
where
    F: FnMut(&str) + Send,
{
    match request.provider.as_str() {
        "claude" => claude::stream_chat(http, request, cancel, on_token).await,
        "gemini" => gemini::stream_chat(http, request, cancel, on_token).await,
        "ollama" => ollama::stream_chat(http, request, cancel, on_token).await,
        _ => openai_compat::stream_chat(http, request, cancel, on_token).await,
    }
}

pub async fn list_models(
    http: &Client,
    provider: &str,
    base_url: &str,
    api_key: &str,
) -> Result<Vec<String>, String> {
    match provider {
        "claude" => claude::list_models(http, base_url, api_key).await,
        "gemini" => gemini::list_models(http, base_url, api_key).await,
        "ollama" => ollama::list_models(http, base_url).await,
        _ => openai_compat::list_models(http, base_url, api_key).await,
    }
}
