use super::stream::{self, Flow};
use super::ChatRequest;
use reqwest::{Client, RequestBuilder};
use serde_json::{json, Value};
use std::sync::atomic::AtomicBool;

const VERSION: &str = "2023-06-01";

fn authenticate(builder: RequestBuilder, api_key: &str) -> RequestBuilder {
    builder
        .header("x-api-key", api_key)
        .header("anthropic-version", VERSION)
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
    let body = json!({
        "model": request.model,
        "max_tokens": request.max_tokens,
        "temperature": request.temperature.min(1.0),
        "system": request.system_prompt,
        "messages": request.messages,
        "stream": true
    });

    let response = authenticate(
        http.post(format!("{}/messages", request.base_url)),
        &request.api_key,
    )
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
        match event["type"].as_str() {
            Some("content_block_delta") => {
                if let Some(text) = event["delta"]["text"].as_str() {
                    answer.push_str(text);
                    on_token(text);
                }
                Ok(Flow::Continue)
            }
            Some("error") => Err(event["error"]["message"]
                .as_str()
                .unwrap_or("erro desconhecido do Claude")
                .to_string()),
            Some("message_stop") => Ok(Flow::Stop),
            _ => Ok(Flow::Continue),
        }
    })
    .await?;

    Ok(answer)
}

pub async fn list_models(
    http: &Client,
    base_url: &str,
    api_key: &str,
) -> Result<Vec<String>, String> {
    let response = authenticate(http.get(format!("{base_url}/models?limit=100")), api_key)
        .send()
        .await
        .map_err(|e| stream::network_error(&e))?;

    let response = stream::check(response).await?;
    let body: Value = response.json().await.map_err(|e| e.to_string())?;

    Ok(body["data"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item["id"].as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default())
}
