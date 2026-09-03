use futures_util::StreamExt;
use reqwest::Response;
use std::sync::atomic::{AtomicBool, Ordering};

pub enum Flow {
    Continue,
    Stop,
}

pub async fn check(response: Response) -> Result<Response, String> {
    if response.status().is_success() {
        return Ok(response);
    }
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    Err(describe_error(status.as_u16(), &body))
}

fn describe_error(status: u16, body: &str) -> String {
    let detail = serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|value| {
            value["error"]["message"]
                .as_str()
                .or_else(|| value["error"].as_str())
                .or_else(|| value["message"].as_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| body.chars().take(200).collect());

    let hint = match status {
        401 | 403 => "API key inválida ou sem permissão. ",
        404 => "Modelo ou endpoint não encontrado. ",
        429 => "Limite de uso atingido. ",
        500..=599 => "O provedor está com problema. ",
        _ => "",
    };

    format!("{hint}HTTP {status}: {detail}")
}

pub async fn for_each_line<F>(
    response: Response,
    cancel: &AtomicBool,
    mut handle: F,
) -> Result<(), String>
where
    F: FnMut(&str) -> Result<Flow, String> + Send,
{
    let mut body = response.bytes_stream();
    let mut buffer: Vec<u8> = Vec::new();

    while let Some(chunk) = body.next().await {
        if cancel.load(Ordering::Relaxed) {
            return Ok(());
        }

        buffer.extend_from_slice(&chunk.map_err(|e| network_error(&e))?);

        while let Some(index) = buffer.iter().position(|byte| *byte == b'\n') {
            let line = buffer.drain(..=index).collect::<Vec<u8>>();
            let line = String::from_utf8_lossy(&line);
            if let Flow::Stop = handle(line.trim_end_matches(['\r', '\n']))? {
                return Ok(());
            }
        }
    }

    if !buffer.is_empty() {
        handle(String::from_utf8_lossy(&buffer).trim())?;
    }
    Ok(())
}

pub fn sse_payload(line: &str) -> Option<&str> {
    let payload = line.strip_prefix("data:")?.trim();
    if payload.is_empty() {
        None
    } else {
        Some(payload)
    }
}

pub fn network_error(error: &reqwest::Error) -> String {
    if error.is_connect() {
        return format!(
            "Não foi possível conectar ao provedor. Verifique a URL base e a conexão. ({error})"
        );
    }
    if error.is_timeout() {
        return "O provedor demorou demais para responder.".to_string();
    }
    error.to_string()
}
