use super::stream::{self, Flow};
use super::{ChatMessage, ChatRequest};
use reqwest::Client;
use serde_json::{json, Value};
use std::sync::atomic::AtomicBool;

pub async fn stream_chat<F>(
    http: &Client,
    request: &ChatRequest,
    cancel: &AtomicBool,
    mut on_token: F,
) -> Result<String, String>
where
    F: FnMut(&str) + Send,
{
    let mut messages = vec![ChatMessage {
        role: "system".into(),
        content: request.system_prompt.clone(),
    }];
    messages.extend(request.messages.iter().cloned());

    let body = json!({
        "model": request.model,
        "messages": messages,
        "stream": true,
        "options": {
            "temperature": request.temperature,
            "top_p": request.top_p,
            "num_predict": request.max_tokens
        }
    });

    let response = http
        .post(format!("{}/api/chat", request.base_url))
        .json(&body)
        .send()
        .await
        .map_err(|e| offline_hint(&e))?;

    let response = stream::check(response).await?;
    let mut answer = String::new();

    stream::for_each_line(response, cancel, |line| {
        if line.trim().is_empty() {
            return Ok(Flow::Continue);
        }

        let event: Value = serde_json::from_str(line).map_err(|e| e.to_string())?;
        if let Some(message) = event["error"].as_str() {
            return Err(message.to_string());
        }
        if let Some(text) = event["message"]["content"].as_str() {
            answer.push_str(text);
            on_token(text);
        }
        if event["done"].as_bool().unwrap_or(false) {
            return Ok(Flow::Stop);
        }
        Ok(Flow::Continue)
    })
    .await?;

    Ok(answer)
}

pub async fn list_models(http: &Client, base_url: &str) -> Result<Vec<String>, String> {
    let response = http
        .get(format!("{base_url}/api/tags"))
        .send()
        .await
        .map_err(|e| offline_hint(&e))?;

    let response = stream::check(response).await?;
    let body: Value = response.json().await.map_err(|e| e.to_string())?;

    let mut models: Vec<String> = body["models"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item["name"].as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    models.sort();
    Ok(models)
}

pub fn offline_hint(error: &reqwest::Error) -> String {
    if error.is_connect() {
        return "Ollama não respondeu. Rode `ollama serve` e tente de novo.".to_string();
    }
    stream::network_error(error)
}
