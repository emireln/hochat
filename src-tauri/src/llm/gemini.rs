use super::stream::{self, Flow};
use super::ChatRequest;
use reqwest::Client;
use serde_json::{json, Value};
use std::sync::atomic::AtomicBool;

fn contents(request: &ChatRequest) -> Vec<Value> {
    request
        .messages
        .iter()
        .map(|message| {
            let role = if message.role == "assistant" {
                "model"
            } else {
                "user"
            };
            json!({ "role": role, "parts": [{ "text": message.content }] })
        })
        .collect()
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
    let url = format!(
        "{}/models/{}:streamGenerateContent?alt=sse",
        request.base_url, request.model
    );

    let body = json!({
        "contents": contents(request),
        "systemInstruction": { "parts": [{ "text": request.system_prompt }] },
        "generationConfig": {
            "temperature": request.temperature,
            "topP": request.top_p,
            "maxOutputTokens": request.max_tokens
        }
    });

    let response = http
        .post(url)
        .header("x-goog-api-key", &request.api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| stream::network_error(&e))?;

    let response = stream::check(response).await?;
    let mut answer = String::new();

    stream::for_each_line(response, cancel, |line| {
        let Some(data) = stream::sse_payload(line) else {
            return Ok(Flow::Continue);
        };

        let event: Value = serde_json::from_str(data).map_err(|e| e.to_string())?;
        if let Some(message) = event["error"]["message"].as_str() {
            return Err(message.to_string());
        }

        if let Some(parts) = event["candidates"][0]["content"]["parts"].as_array() {
            for part in parts {
                if let Some(text) = part["text"].as_str() {
                    answer.push_str(text);
                    on_token(text);
                }
            }
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
        .get(format!("{base_url}/models?pageSize=200"))
        .header("x-goog-api-key", api_key)
        .send()
        .await
        .map_err(|e| stream::network_error(&e))?;

    let response = stream::check(response).await?;
    let body: Value = response.json().await.map_err(|e| e.to_string())?;

    Ok(body["models"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter(|item| {
                    item["supportedGenerationMethods"]
                        .as_array()
                        .map(|methods| methods.iter().any(|m| m == "generateContent"))
                        .unwrap_or(false)
                })
                .filter_map(|item| item["name"].as_str())
                .map(|name| name.trim_start_matches("models/").to_string())
                .collect()
        })
        .unwrap_or_default())
}
