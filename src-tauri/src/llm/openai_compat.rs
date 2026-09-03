use super::stream::{self, Flow};
use super::{ChatMessage, ChatRequest};
use reqwest::Client;
use serde_json::{json, Value};
use std::sync::atomic::AtomicBool;

fn payload(request: &ChatRequest) -> Value {
    let mut messages = vec![ChatMessage {
        role: "system".into(),
        content: request.system_prompt.clone(),
    }];
    messages.extend(request.messages.iter().cloned());

    let mut body = json!({
        "model": request.model,
        "messages": messages,
        "stream": true
    });

    body[&request.max_tokens_field] = json!(request.max_tokens);

    if request.supports_sampling {
        body["temperature"] = json!(request.temperature);
        body["top_p"] = json!(request.top_p);
    }

    body
}

pub async fn stream_chat<F>(
    http: &Client,
    request: &ChatRequest,
    cancel: &AtomicBool,
    mut on_token: F,
) -> Result<String, String>
where
    F: FnMut(&str) + Send,
{
    let response = http
        .post(format!("{}/chat/completions", request.base_url))
        .bearer_auth(&request.api_key)
        .json(&payload(request))
        .send()
        .await
        .map_err(|e| stream::network_error(&e))?;

    let response = stream::check(response).await?;
    let mut answer = String::new();

    stream::for_each_line(response, cancel, |line| {
        let Some(data) = stream::sse_payload(line) else {
            return Ok(Flow::Continue);
        };
        if data == "[DONE]" {
            return Ok(Flow::Stop);
        }

        let event: Value = serde_json::from_str(data).map_err(|e| e.to_string())?;
        if let Some(message) = event["error"]["message"].as_str() {
            return Err(message.to_string());
        }
        if let Some(text) = event["choices"][0]["delta"]["content"].as_str() {
            answer.push_str(text);
            on_token(text);
        }
        Ok(Flow::Continue)
    })
    .await?;

    Ok(answer)
}

pub async fn list_models(
    http: &Client,
    base_url: &str,
    api_key: &str,
) -> Result<Vec<String>, String> {
    let response = http
        .get(format!("{base_url}/models"))
        .bearer_auth(api_key)
        .send()
        .await
        .map_err(|e| stream::network_error(&e))?;

    let response = stream::check(response).await?;
    let body: Value = response.json().await.map_err(|e| e.to_string())?;

    let mut models: Vec<String> = body["data"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item["id"].as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    models.sort();
    Ok(models)
}
