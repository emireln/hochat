use crate::llm::{ollama, stream};
use crate::settings::EmbedConfig;
use reqwest::Client;
use serde_json::{json, Value};

pub const BATCH: usize = 16;

pub async fn embed_many(
    http: &Client,
    config: &EmbedConfig,
    texts: &[String],
) -> Result<Vec<Vec<f32>>, String> {
    if texts.is_empty() {
        return Ok(Vec::new());
    }

    let vectors = match config.provider.as_str() {
        "ollama" => ollama_embed(http, config, texts).await?,
        "gemini" => gemini_embed(http, config, texts).await?,
        _ => openai_embed(http, config, texts).await?,
    };

    if vectors.len() != texts.len() {
        return Err(format!(
            "O provedor devolveu {} embeddings para {} trechos.",
            vectors.len(),
            texts.len()
        ));
    }
    if vectors.iter().any(|vector| vector.is_empty()) {
        return Err("O provedor devolveu um embedding vazio.".to_string());
    }

    Ok(vectors)
}

pub async fn embed_one(
    http: &Client,
    config: &EmbedConfig,
    text: &str,
) -> Result<Vec<f32>, String> {
    let mut vectors = embed_many(http, config, &[text.to_string()]).await?;
    Ok(vectors.remove(0))
}

fn numbers(value: &Value) -> Vec<f32> {
    value
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_f64().map(|n| n as f32))
                .collect()
        })
        .unwrap_or_default()
}

async fn ollama_embed(
    http: &Client,
    config: &EmbedConfig,
    texts: &[String],
) -> Result<Vec<Vec<f32>>, String> {
    let response = http
        .post(format!("{}/api/embed", config.base_url))
        .json(&json!({ "model": config.model, "input": texts }))
        .send()
        .await
        .map_err(|e| ollama::offline_hint(&e))?;

    if response.status() == 404 {
        return ollama_embed_legacy(http, config, texts).await;
    }

    let response = stream::check(response).await.map_err(model_hint)?;
    let body: Value = response.json().await.map_err(|e| e.to_string())?;

    Ok(body["embeddings"]
        .as_array()
        .map(|items| items.iter().map(numbers).collect())
        .unwrap_or_default())
}

async fn ollama_embed_legacy(
    http: &Client,
    config: &EmbedConfig,
    texts: &[String],
) -> Result<Vec<Vec<f32>>, String> {
    let mut vectors = Vec::with_capacity(texts.len());

    for text in texts {
        let response = http
            .post(format!("{}/api/embeddings", config.base_url))
            .json(&json!({ "model": config.model, "prompt": text }))
            .send()
            .await
            .map_err(|e| ollama::offline_hint(&e))?;

        let response = stream::check(response).await.map_err(model_hint)?;
        let body: Value = response.json().await.map_err(|e| e.to_string())?;
        vectors.push(numbers(&body["embedding"]));
    }

    Ok(vectors)
}

async fn openai_embed(
    http: &Client,
    config: &EmbedConfig,
    texts: &[String],
) -> Result<Vec<Vec<f32>>, String> {
    let response = http
        .post(format!("{}/embeddings", config.base_url))
        .bearer_auth(&config.api_key)
        .json(&json!({ "model": config.model, "input": texts }))
        .send()
        .await
        .map_err(|e| stream::network_error(&e))?;

    let response = stream::check(response).await?;
    let body: Value = response.json().await.map_err(|e| e.to_string())?;

    let mut items: Vec<(usize, Vec<f32>)> = body["data"]
        .as_array()
        .map(|entries| {
            entries
                .iter()
                .enumerate()
                .map(|(fallback, entry)| {
                    let index = entry["index"].as_u64().map(|n| n as usize).unwrap_or(fallback);
                    (index, numbers(&entry["embedding"]))
                })
                .collect()
        })
        .unwrap_or_default();

    items.sort_by_key(|(index, _)| *index);
    Ok(items.into_iter().map(|(_, vector)| vector).collect())
}

async fn gemini_embed(
    http: &Client,
    config: &EmbedConfig,
    texts: &[String],
) -> Result<Vec<Vec<f32>>, String> {
    let requests: Vec<Value> = texts
        .iter()
        .map(|text| {
            json!({
                "model": format!("models/{}", config.model),
                "content": { "parts": [{ "text": text }] }
            })
        })
        .collect();

    let response = http
        .post(format!(
            "{}/models/{}:batchEmbedContents",
            config.base_url, config.model
        ))
        .header("x-goog-api-key", &config.api_key)
        .json(&json!({ "requests": requests }))
        .send()
        .await
        .map_err(|e| stream::network_error(&e))?;

    let response = stream::check(response).await?;
    let body: Value = response.json().await.map_err(|e| e.to_string())?;

    Ok(body["embeddings"]
        .as_array()
        .map(|items| items.iter().map(|item| numbers(&item["values"])).collect())
        .unwrap_or_default())
}

fn model_hint(error: String) -> String {
    if error.contains("404") {
        return format!("{error} Baixe o modelo com `ollama pull nomic-embed-text`.");
    }
    error
}
