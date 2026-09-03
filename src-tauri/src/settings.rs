use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub struct ProviderMeta {
    pub id: &'static str,
    pub label: &'static str,
    pub base_url: &'static str,
    pub default_model: &'static str,
    pub needs_key: bool,
    pub key_url: &'static str,
    pub fallback_models: &'static [&'static str],
    /// A familia GPT-5 so aceita a temperatura padrao e usa max_completion_tokens.
    pub supports_sampling: bool,
    pub max_tokens_field: &'static str,
}

pub const PROVIDERS: &[ProviderMeta] = &[
    ProviderMeta {
        id: "openai",
        label: "OpenAI",
        base_url: "https://api.openai.com/v1",
        default_model: "gpt-5.6-terra",
        needs_key: true,
        key_url: "https://platform.openai.com/api-keys",
        fallback_models: &[
            "gpt-5.6-sol",
            "gpt-5.6-terra",
            "gpt-5.6-luna",
            "gpt-5.5",
            "gpt-5.4-mini",
        ],
        supports_sampling: false,
        max_tokens_field: "max_completion_tokens",
    },
    ProviderMeta {
        id: "claude",
        label: "Claude",
        base_url: "https://api.anthropic.com/v1",
        default_model: "claude-sonnet-5",
        needs_key: true,
        key_url: "https://console.anthropic.com/settings/keys",
        fallback_models: &[
            "claude-sonnet-5",
            "claude-opus-5",
            "claude-haiku-4-5",
            "claude-fable-5-1",
        ],
        supports_sampling: true,
        max_tokens_field: "max_tokens",
    },
    ProviderMeta {
        id: "gemini",
        label: "Gemini",
        base_url: "https://generativelanguage.googleapis.com/v1beta",
        default_model: "gemini-3.7-flash",
        needs_key: true,
        key_url: "https://aistudio.google.com/apikey",
        fallback_models: &[
            "gemini-3.8-flash",
            "gemini-3.7-flash",
            "gemini-3.6-flash",
            "gemini-3.5-flash-lite",
            "gemini-3.1-pro-preview",
        ],
        supports_sampling: true,
        max_tokens_field: "max_tokens",
    },
    ProviderMeta {
        id: "deepseek",
        label: "DeepSeek",
        base_url: "https://api.deepseek.com/v1",
        default_model: "deepseek-v4-flash",
        needs_key: true,
        key_url: "https://platform.deepseek.com/api_keys",
        fallback_models: &["deepseek-v4-flash", "deepseek-v4-pro"],
        supports_sampling: true,
        max_tokens_field: "max_tokens",
    },
    ProviderMeta {
        id: "glm",
        label: "GLM",
        base_url: "https://open.bigmodel.cn/api/paas/v4",
        default_model: "glm-5.3",
        needs_key: true,
        key_url: "https://open.bigmodel.cn/usercenter/apikeys",
        fallback_models: &["glm-5.3", "glm-5.3-flash", "glm-5.2", "glm-5-turbo"],
        supports_sampling: true,
        max_tokens_field: "max_tokens",
    },
    ProviderMeta {
        id: "ollama",
        label: "Ollama",
        base_url: "http://127.0.0.1:11434",
        default_model: "qwen3",
        needs_key: false,
        key_url: "https://ollama.com/download",
        fallback_models: &["qwen3", "llama3.3", "gemma3", "gpt-oss", "deepseek-r1"],
        supports_sampling: true,
        max_tokens_field: "max_tokens",
    },
];

pub struct EmbedMeta {
    pub id: &'static str,
    pub default_model: &'static str,
    pub models: &'static [&'static str],
}

pub const EMBED_PROVIDERS: &[EmbedMeta] = &[
    EmbedMeta {
        id: "ollama",
        default_model: "nomic-embed-text",
        models: &[
            "nomic-embed-text",
            "bge-m3",
            "qwen3-embedding",
            "mxbai-embed-large",
            "embeddinggemma",
        ],
    },
    EmbedMeta {
        id: "openai",
        default_model: "text-embedding-3-small",
        models: &["text-embedding-3-small", "text-embedding-3-large"],
    },
    EmbedMeta {
        id: "gemini",
        default_model: "gemini-embedding-001",
        models: &["gemini-embedding-001"],
    },
];

pub const DEFAULT_SYSTEM_PROMPT: &str =
    "Você é o HoChat, um assistente direto e objetivo. Responda em português do Brasil.";

pub fn meta(provider_id: &str) -> Option<&'static ProviderMeta> {
    PROVIDERS.iter().find(|provider| provider.id == provider_id)
}

pub fn embed_meta(provider_id: &str) -> Option<&'static EmbedMeta> {
    EMBED_PROVIDERS
        .iter()
        .find(|provider| provider.id == provider_id)
}

pub fn get(conn: &Connection, key: &str) -> Option<String> {
    conn.query_row("SELECT value FROM settings WHERE key = ?1", [key], |row| {
        row.get::<_, String>(0)
    })
    .ok()
    .filter(|value| !value.is_empty())
}

pub fn get_or(conn: &Connection, key: &str, fallback: &str) -> String {
    get(conn, key).unwrap_or_else(|| fallback.to_string())
}

fn number<T: std::str::FromStr>(conn: &Connection, key: &str, fallback: T) -> T {
    get(conn, key)
        .and_then(|value| value.parse().ok())
        .unwrap_or(fallback)
}

fn flag(conn: &Connection, key: &str, fallback: bool) -> bool {
    get(conn, key)
        .map(|value| value == "true")
        .unwrap_or(fallback)
}

pub fn set(conn: &Connection, key: &str, value: &str) -> Result<(), String> {
    if value.is_empty() {
        conn.execute("DELETE FROM settings WHERE key = ?1", [key])
            .map_err(|e| e.to_string())?;
        return Ok(());
    }
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        [key, value],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn mask(key: &str) -> String {
    let visible: String = key
        .chars()
        .rev()
        .take(4)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    format!("{}{}", "•".repeat(8), visible)
}

pub fn api_key(conn: &Connection, provider_id: &str) -> String {
    get_or(conn, &format!("api_key.{provider_id}"), "")
}

pub fn base_url(conn: &Connection, provider_id: &str) -> String {
    let fallback = meta(provider_id).map(|m| m.base_url).unwrap_or_default();
    get_or(conn, &format!("base_url.{provider_id}"), fallback)
        .trim_end_matches('/')
        .to_string()
}

pub fn model_for(conn: &Connection, provider_id: &str) -> String {
    let fallback = meta(provider_id)
        .map(|m| m.default_model)
        .unwrap_or_default();
    get_or(conn, &format!("model.{provider_id}"), fallback)
}

pub fn embed_model_for(conn: &Connection, provider_id: &str) -> String {
    let fallback = embed_meta(provider_id)
        .map(|m| m.default_model)
        .unwrap_or_default();
    get_or(conn, &format!("embed_model.{provider_id}"), fallback)
}

pub struct ChatConfig {
    pub provider: String,
    pub model: String,
    pub base_url: String,
    pub api_key: String,
    pub system_prompt: String,
    pub temperature: f32,
    pub top_p: f32,
    pub max_tokens: u32,
    pub history_limit: usize,
}

pub fn chat_config(conn: &Connection) -> Result<ChatConfig, String> {
    let provider = get_or(conn, "provider", "ollama");
    let meta = meta(&provider).ok_or_else(|| format!("Provedor desconhecido: {provider}"))?;
    let api_key = api_key(conn, &provider);

    if meta.needs_key && api_key.is_empty() {
        return Err(format!(
            "Configure a API key de {} em Configurações.",
            meta.label
        ));
    }

    Ok(ChatConfig {
        model: model_for(conn, &provider),
        base_url: base_url(conn, &provider),
        api_key,
        system_prompt: get_or(conn, "system_prompt", DEFAULT_SYSTEM_PROMPT),
        temperature: number(conn, "temperature", 0.7),
        top_p: number(conn, "top_p", 1.0),
        max_tokens: number(conn, "max_tokens", 4096),
        history_limit: number(conn, "history_limit", 20),
        provider,
    })
}

pub struct RagConfig {
    pub top_k: usize,
    pub min_score: f32,
    pub chunk_size: usize,
    pub chunk_overlap: usize,
}

pub fn rag_config(conn: &Connection) -> RagConfig {
    RagConfig {
        top_k: number(conn, "rag_top_k", 4),
        min_score: number(conn, "rag_min_score", 0.15),
        chunk_size: number(conn, "chunk_size", 2000),
        chunk_overlap: number(conn, "chunk_overlap", 200),
    }
}

pub struct EmbedConfig {
    pub provider: String,
    pub model: String,
    pub base_url: String,
    pub api_key: String,
}

impl EmbedConfig {
    pub fn tag(&self) -> String {
        format!("{}:{}", self.provider, self.model)
    }
}

pub fn embed_config(conn: &Connection) -> Result<EmbedConfig, String> {
    let provider = get_or(conn, "embed_provider", "ollama");
    let meta = meta(&provider).ok_or_else(|| format!("Provedor desconhecido: {provider}"))?;
    let api_key = api_key(conn, &provider);

    if meta.needs_key && api_key.is_empty() {
        return Err(format!(
            "Os embeddings usam {}, mas a API key não está configurada.",
            meta.label
        ));
    }

    Ok(EmbedConfig {
        model: embed_model_for(conn, &provider),
        base_url: base_url(conn, &provider),
        api_key,
        provider,
    })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderView {
    pub id: String,
    pub label: String,
    pub base_url: String,
    pub default_base_url: String,
    pub model: String,
    pub models: Vec<String>,
    pub needs_key: bool,
    pub has_key: bool,
    pub key_masked: String,
    pub key_url: String,
    pub supports_sampling: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmbedProviderView {
    pub id: String,
    pub label: String,
    pub models: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsView {
    pub provider: String,
    pub providers: Vec<ProviderView>,
    pub embed_provider: String,
    pub embed_model: String,
    pub embed_providers: Vec<EmbedProviderView>,
    pub system_prompt: String,
    pub temperature: f32,
    pub top_p: f32,
    pub max_tokens: i64,
    pub history_limit: i64,
    pub rag_top_k: i64,
    pub rag_min_score: f32,
    pub chunk_size: i64,
    pub chunk_overlap: i64,
    pub theme: String,
    pub send_on_enter: bool,
}

pub fn view(conn: &Connection) -> Result<SettingsView, String> {
    let embed_provider = get_or(conn, "embed_provider", "ollama");
    let rag = rag_config(conn);

    let providers = PROVIDERS
        .iter()
        .map(|meta| {
            let key = api_key(conn, meta.id);
            ProviderView {
                id: meta.id.to_string(),
                label: meta.label.to_string(),
                base_url: base_url(conn, meta.id),
                default_base_url: meta.base_url.to_string(),
                model: model_for(conn, meta.id),
                models: meta.fallback_models.iter().map(|m| m.to_string()).collect(),
                needs_key: meta.needs_key,
                has_key: !key.is_empty(),
                key_masked: if key.is_empty() {
                    String::new()
                } else {
                    mask(&key)
                },
                key_url: meta.key_url.to_string(),
                supports_sampling: meta.supports_sampling,
            }
        })
        .collect();

    let embed_providers = EMBED_PROVIDERS
        .iter()
        .map(|embed| EmbedProviderView {
            id: embed.id.to_string(),
            label: meta(embed.id)
                .map(|m| m.label.to_string())
                .unwrap_or_else(|| embed.id.to_string()),
            models: embed.models.iter().map(|m| m.to_string()).collect(),
        })
        .collect();

    Ok(SettingsView {
        provider: get_or(conn, "provider", "ollama"),
        providers,
        embed_model: embed_model_for(conn, &embed_provider),
        embed_provider,
        embed_providers,
        system_prompt: get_or(conn, "system_prompt", DEFAULT_SYSTEM_PROMPT),
        temperature: number(conn, "temperature", 0.7),
        top_p: number(conn, "top_p", 1.0),
        max_tokens: number(conn, "max_tokens", 4096),
        history_limit: number(conn, "history_limit", 20),
        rag_top_k: rag.top_k as i64,
        rag_min_score: rag.min_score,
        chunk_size: rag.chunk_size as i64,
        chunk_overlap: rag.chunk_overlap as i64,
        theme: get_or(conn, "theme", "system"),
        send_on_enter: flag(conn, "send_on_enter", true),
    })
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SettingsPatch {
    pub provider: Option<String>,
    pub embed_provider: Option<String>,
    pub embed_model: Option<String>,
    pub system_prompt: Option<String>,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub max_tokens: Option<i64>,
    pub history_limit: Option<i64>,
    pub rag_top_k: Option<i64>,
    pub rag_min_score: Option<f32>,
    pub chunk_size: Option<i64>,
    pub chunk_overlap: Option<i64>,
    pub theme: Option<String>,
    pub send_on_enter: Option<bool>,
    pub models: Option<HashMap<String, String>>,
    pub api_keys: Option<HashMap<String, String>>,
    pub base_urls: Option<HashMap<String, String>>,
}

pub fn apply(conn: &Connection, patch: SettingsPatch) -> Result<(), String> {
    if let Some(provider) = patch.provider {
        set(conn, "provider", &provider)?;
    }
    if let Some(provider) = patch.embed_provider {
        set(conn, "embed_provider", &provider)?;
    }
    if let Some(model) = patch.embed_model {
        let provider = get_or(conn, "embed_provider", "ollama");
        set(conn, &format!("embed_model.{provider}"), model.trim())?;
    }
    if let Some(prompt) = patch.system_prompt {
        set(conn, "system_prompt", prompt.trim())?;
    }
    if let Some(value) = patch.temperature {
        set(conn, "temperature", &value.clamp(0.0, 2.0).to_string())?;
    }
    if let Some(value) = patch.top_p {
        set(conn, "top_p", &value.clamp(0.1, 1.0).to_string())?;
    }
    if let Some(value) = patch.max_tokens {
        set(conn, "max_tokens", &value.clamp(256, 32768).to_string())?;
    }
    if let Some(value) = patch.history_limit {
        set(conn, "history_limit", &value.clamp(2, 100).to_string())?;
    }
    if let Some(value) = patch.rag_top_k {
        set(conn, "rag_top_k", &value.clamp(1, 12).to_string())?;
    }
    if let Some(value) = patch.rag_min_score {
        set(conn, "rag_min_score", &value.clamp(0.0, 0.9).to_string())?;
    }
    if let Some(value) = patch.chunk_size {
        set(conn, "chunk_size", &value.clamp(500, 8000).to_string())?;
    }
    if let Some(value) = patch.chunk_overlap {
        set(conn, "chunk_overlap", &value.clamp(0, 1000).to_string())?;
    }
    if let Some(theme) = patch.theme {
        set(conn, "theme", &theme)?;
    }
    if let Some(value) = patch.send_on_enter {
        set(conn, "send_on_enter", if value { "true" } else { "false" })?;
    }
    for (provider, model) in patch.models.unwrap_or_default() {
        set(conn, &format!("model.{provider}"), model.trim())?;
    }
    for (provider, key) in patch.api_keys.unwrap_or_default() {
        set(conn, &format!("api_key.{provider}"), key.trim())?;
    }
    for (provider, url) in patch.base_urls.unwrap_or_default() {
        let url = url.trim().trim_end_matches('/');
        let is_default = meta(&provider).map(|m| m.base_url == url).unwrap_or(false);
        set(
            conn,
            &format!("base_url.{provider}"),
            if is_default { "" } else { url },
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::memory;

    fn patch_with(entries: [(&str, &str); 1]) -> HashMap<String, String> {
        entries
            .into_iter()
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect()
    }

    #[test]
    fn chave_so_mostra_os_ultimos_quatro() {
        assert_eq!(mask("sk-proj-abcdefgh1234"), "••••••••1234");
    }

    #[test]
    fn cada_provedor_lembra_o_proprio_modelo() {
        let conn = memory();
        apply(
            &conn,
            SettingsPatch {
                models: Some(patch_with([("openai", "gpt-5.6-sol")])),
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(model_for(&conn, "openai"), "gpt-5.6-sol");
        assert_eq!(model_for(&conn, "claude"), "claude-sonnet-5");
    }

    #[test]
    fn provedor_pago_sem_chave_nao_deixa_conversar() {
        let conn = memory();
        set(&conn, "provider", "openai").unwrap();

        assert!(chat_config(&conn).is_err());

        apply(
            &conn,
            SettingsPatch {
                api_keys: Some(patch_with([("openai", "sk-teste")])),
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(chat_config(&conn).unwrap().api_key, "sk-teste");
    }

    #[test]
    fn ollama_conversa_sem_chave_nenhuma() {
        let conn = memory();
        let config = chat_config(&conn).unwrap();

        assert_eq!(config.provider, "ollama");
        assert_eq!(config.base_url, "http://127.0.0.1:11434");
    }

    #[test]
    fn url_base_igual_ao_padrao_nao_ocupa_espaco() {
        let conn = memory();
        apply(
            &conn,
            SettingsPatch {
                base_urls: Some(patch_with([("openai", "https://api.openai.com/v1/")])),
                ..Default::default()
            },
        )
        .unwrap();

        assert!(get(&conn, "base_url.openai").is_none());
        assert_eq!(base_url(&conn, "openai"), "https://api.openai.com/v1");
    }

    #[test]
    fn a_tela_nunca_recebe_a_chave_inteira() {
        let conn = memory();
        apply(
            &conn,
            SettingsPatch {
                api_keys: Some(patch_with([("openai", "sk-proj-abcdefgh1234")])),
                ..Default::default()
            },
        )
        .unwrap();

        let openai = view(&conn)
            .unwrap()
            .providers
            .into_iter()
            .find(|provider| provider.id == "openai")
            .unwrap();

        assert!(openai.has_key);
        assert_eq!(openai.key_masked, "••••••••1234");
        assert!(!serde_json::to_string(&openai)
            .unwrap()
            .contains("abcdefgh"));
    }

    #[test]
    fn valores_fora_do_limite_sao_cortados() {
        let conn = memory();
        apply(
            &conn,
            SettingsPatch {
                temperature: Some(9.0),
                max_tokens: Some(999_999),
                rag_top_k: Some(0),
                ..Default::default()
            },
        )
        .unwrap();

        let view = view(&conn).unwrap();
        assert_eq!(view.temperature, 2.0);
        assert_eq!(view.max_tokens, 32768);
        assert_eq!(view.rag_top_k, 1);
    }
}
