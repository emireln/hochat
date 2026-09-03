use crate::db::{self, Chat, Document, Message};
use crate::llm::{self, ChatMessage, ChatRequest};
use crate::rag::{self, chunk, embed, ingest, search};
use tauri::Manager;
use tauri_plugin_opener::OpenerExt;
use crate::settings::{self, SettingsPatch, SettingsView};
use reqwest::Client;
use rusqlite::Connection;
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter, State};

pub const NEW_CHAT_TITLE: &str = "Nova conversa";

pub struct AppState {
    db: Mutex<Connection>,
    http: Client,
    running: Mutex<HashMap<i64, Arc<AtomicBool>>>,
}

impl AppState {
    pub fn new(connection: Connection) -> Self {
        let http = Client::builder()
            .connect_timeout(Duration::from_secs(15))
            .user_agent("HoChat/0.1")
            .build()
            .expect("cliente HTTP");

        Self {
            db: Mutex::new(connection),
            http,
            running: Mutex::new(HashMap::new()),
        }
    }
}

fn read<T>(state: &AppState, action: impl FnOnce(&Connection) -> Result<T, String>) -> Result<T, String> {
    let connection = state.db.lock().map_err(|_| "Banco de dados ocupado.".to_string())?;
    action(&connection)
}

fn write<T>(
    state: &AppState,
    action: impl FnOnce(&mut Connection) -> Result<T, String>,
) -> Result<T, String> {
    let mut connection = state.db.lock().map_err(|_| "Banco de dados ocupado.".to_string())?;
    action(&mut connection)
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct TokenEvent {
    chat_id: i64,
    text: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct TitleEvent {
    chat_id: i64,
    title: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SourcesEvent {
    chat_id: i64,
    hits: Vec<search::Hit>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct IngestEvent {
    name: String,
    stage: String,
    done: usize,
    total: usize,
}

#[tauri::command]
pub fn list_chats(state: State<'_, AppState>) -> Result<Vec<Chat>, String> {
    read(&state, db::list_chats)
}

#[tauri::command]
pub fn create_chat(state: State<'_, AppState>) -> Result<Chat, String> {
    read(&state, |connection| db::create_chat(connection, NEW_CHAT_TITLE))
}

#[tauri::command]
pub fn rename_chat(state: State<'_, AppState>, chat_id: i64, title: String) -> Result<(), String> {
    let title = title.trim();
    if title.is_empty() {
        return Err("O título não pode ficar vazio.".to_string());
    }
    read(&state, |connection| db::rename_chat(connection, chat_id, title))
}

#[tauri::command]
pub fn delete_chat(state: State<'_, AppState>, chat_id: i64) -> Result<(), String> {
    read(&state, |connection| db::delete_chat(connection, chat_id))
}

#[tauri::command]
pub fn list_messages(state: State<'_, AppState>, chat_id: i64) -> Result<Vec<Message>, String> {
    read(&state, |connection| db::list_messages(connection, chat_id))
}

#[tauri::command]
pub fn stop_generation(state: State<'_, AppState>, chat_id: i64) {
    if let Ok(running) = state.running.lock() {
        if let Some(flag) = running.get(&chat_id) {
            flag.store(true, Ordering::Relaxed);
        }
    }
}

#[tauri::command]
pub async fn send_message(
    app: AppHandle,
    state: State<'_, AppState>,
    chat_id: i64,
    content: String,
    use_rag: bool,
) -> Result<Message, String> {
    let content = content.trim().to_string();
    if content.is_empty() {
        return Err("Escreva alguma coisa antes de enviar.".to_string());
    }

    let config = read(&state, settings::chat_config)?;
    let user_message = read(&state, |connection| {
        db::insert_message(connection, chat_id, "user", &content, &[])
    })?;

    if let Some(title) = read(&state, |connection| {
        if db::chat_title(connection, chat_id)? != NEW_CHAT_TITLE {
            return Ok(None);
        }
        let title = title_from(&content);
        db::rename_chat(connection, chat_id, &title)?;
        Ok(Some(title))
    })? {
        let _ = app.emit("chat-title", TitleEvent { chat_id, title });
    }

    let mut sources: Vec<String> = Vec::new();
    let mut context = String::new();

    if use_rag {
        let embed_config = read(&state, settings::embed_config)?;
        let vector = embed::embed_one(&state.http, &embed_config, &content).await?;
        let tag = embed_config.tag();
        let hits = read(&state, |connection| {
            let rag = settings::rag_config(connection);
            search::top_k(connection, &vector, &tag, rag.top_k, rag.min_score)
        })?;

        if !hits.is_empty() {
            context = rag::build_context(&hits);
            for hit in &hits {
                if !sources.contains(&hit.document) {
                    sources.push(hit.document.clone());
                }
            }
            let _ = app.emit("chat-sources", SourcesEvent { chat_id, hits });
        }
    }

    let history = read(&state, |connection| db::list_messages(connection, chat_id))?;
    let meta = settings::meta(&config.provider)
        .ok_or_else(|| format!("Provedor desconhecido: {}", config.provider))?;

    let window = recent_history(&history, config.history_limit);

    let request = ChatRequest {
        provider: config.provider,
        base_url: config.base_url,
        api_key: config.api_key,
        model: config.model,
        system_prompt: if context.is_empty() {
            config.system_prompt
        } else {
            format!("{}\n\n{}", config.system_prompt, context)
        },
        messages: window
            .iter()
            .map(|message| ChatMessage {
                role: message.role.clone(),
                content: message.content.clone(),
            })
            .collect(),
        temperature: config.temperature,
        top_p: config.top_p,
        max_tokens: config.max_tokens,
        supports_sampling: meta.supports_sampling,
        max_tokens_field: meta.max_tokens_field.to_string(),
    };

    let cancel = Arc::new(AtomicBool::new(false));
    {
        let mut running = state.running.lock().map_err(|_| "Estado ocupado.".to_string())?;
        running.insert(chat_id, cancel.clone());
    }

    let emitter = app.clone();
    let result = llm::stream_chat(&state.http, &request, &cancel, |text| {
        let _ = emitter.emit(
            "chat-token",
            TokenEvent {
                chat_id,
                text: text.to_string(),
            },
        );
    })
    .await;

    if let Ok(mut running) = state.running.lock() {
        running.remove(&chat_id);
    }

    let answer = match result {
        Ok(answer) => answer,
        Err(error) => {
            let _ = read(&state, |connection| {
                db::delete_message(connection, user_message.id)
            });
            return Err(error);
        }
    };

    let answer = if answer.trim().is_empty() {
        "(sem resposta)".to_string()
    } else {
        answer
    };

    read(&state, |connection| {
        db::insert_message(connection, chat_id, "assistant", &answer, &sources)
    })
}

fn recent_history(history: &[Message], limit: usize) -> &[Message] {
    let start = history.len().saturating_sub(limit.max(1));
    let mut window = &history[start..];

    while window.first().is_some_and(|message| message.role != "user") {
        window = &window[1..];
    }

    window
}

fn title_from(content: &str) -> String {
    let clean = content.split_whitespace().collect::<Vec<_>>().join(" ");
    if clean.chars().count() <= 42 {
        return clean;
    }

    let mut cut: String = clean.chars().take(42).collect();
    if let Some(index) = cut.rfind(' ') {
        cut.truncate(index);
    }
    format!("{cut}...")
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelList {
    pub models: Vec<String>,
    pub live: bool,
    pub message: String,
}

#[tauri::command]
pub async fn list_models(
    state: State<'_, AppState>,
    provider: String,
) -> Result<ModelList, String> {
    let meta = settings::meta(&provider)
        .ok_or_else(|| format!("Provedor desconhecido: {provider}"))?;

    let (base_url, api_key) = read(&state, |connection| {
        Ok((
            settings::base_url(connection, &provider),
            settings::api_key(connection, &provider),
        ))
    })?;

    let fallback = |message: String| ModelList {
        models: meta.fallback_models.iter().map(|m| m.to_string()).collect(),
        live: false,
        message,
    };

    if meta.needs_key && api_key.is_empty() {
        return Ok(fallback("Lista padrão: falta a API key.".to_string()));
    }

    match llm::list_models(&state.http, &provider, &base_url, &api_key).await {
        Ok(models) if !models.is_empty() => Ok(ModelList {
            message: format!("{} modelos disponíveis.", models.len()),
            models,
            live: true,
        }),
        Ok(_) => Ok(fallback("O provedor não listou modelos.".to_string())),
        Err(error) => Ok(fallback(error)),
    }
}

#[tauri::command]
pub async fn check_provider(state: State<'_, AppState>, provider: String) -> Result<String, String> {
    let (base_url, api_key) = read(&state, |connection| {
        Ok((
            settings::base_url(connection, &provider),
            settings::api_key(connection, &provider),
        ))
    })?;

    let models = llm::list_models(&state.http, &provider, &base_url, &api_key).await?;
    Ok(format!("Conexão ok. {} modelos encontrados.", models.len()))
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Result<SettingsView, String> {
    read(&state, settings::view)
}

#[tauri::command]
pub fn save_settings(
    state: State<'_, AppState>,
    patch: SettingsPatch,
) -> Result<SettingsView, String> {
    read(&state, |connection| {
        settings::apply(connection, patch)?;
        settings::view(connection)
    })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RagStatus {
    pub ready: bool,
    pub message: String,
    pub provider: String,
    pub model: String,
    pub documents: usize,
    pub chunks: i64,
    pub supported: Vec<String>,
}

#[tauri::command]
pub fn rag_status(state: State<'_, AppState>) -> Result<RagStatus, String> {
    read(&state, |connection| {
        let supported = ingest::SUPPORTED.iter().map(|e| e.to_string()).collect();
        let documents = db::list_documents(connection)?.len();

        match settings::embed_config(connection) {
            Ok(config) => Ok(RagStatus {
                ready: true,
                message: String::new(),
                chunks: db::count_chunks(connection, &config.tag())?,
                provider: config.provider,
                model: config.model,
                documents,
                supported,
            }),
            Err(message) => Ok(RagStatus {
                ready: false,
                message,
                provider: settings::get_or(connection, "embed_provider", "ollama"),
                model: String::new(),
                documents,
                chunks: 0,
                supported,
            }),
        }
    })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageInfo {
    pub folder: String,
    pub size_bytes: i64,
    pub chats: i64,
    pub messages: i64,
    pub documents: i64,
    pub chunks: i64,
}

#[tauri::command]
pub fn storage_info(app: AppHandle, state: State<'_, AppState>) -> Result<StorageInfo, String> {
    let folder = data_folder(&app)?;
    let size_bytes = std::fs::metadata(folder.join("hochat.db"))
        .map(|data| data.len() as i64)
        .unwrap_or(0);

    read(&state, |connection| {
        let count = |table: &str| -> Result<i64, String> {
            connection
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| row.get(0))
                .map_err(|e| e.to_string())
        };

        Ok(StorageInfo {
            folder: folder.to_string_lossy().into_owned(),
            size_bytes,
            chats: count("chats")?,
            messages: count("messages")?,
            documents: count("documents")?,
            chunks: count("chunks")?,
        })
    })
}

fn data_folder(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map_err(|e| format!("Não achei a pasta de dados: {e}"))
}

#[tauri::command]
pub fn open_data_folder(app: AppHandle) -> Result<(), String> {
    let folder = data_folder(&app)?;
    app.opener()
        .open_path(folder.to_string_lossy(), None::<&str>)
        .map_err(|e| format!("Não consegui abrir a pasta: {e}"))
}

#[tauri::command]
pub fn clear_all_chats(state: State<'_, AppState>) -> Result<(), String> {
    read(&state, |connection| {
        connection
            .execute("DELETE FROM chats", [])
            .map_err(|e| e.to_string())?;
        Ok(())
    })
}

#[tauri::command]
pub fn list_documents(state: State<'_, AppState>) -> Result<Vec<Document>, String> {
    read(&state, db::list_documents)
}

#[tauri::command]
pub fn delete_document(state: State<'_, AppState>, document_id: i64) -> Result<(), String> {
    read(&state, |connection| {
        db::delete_document(connection, document_id)
    })
}

#[tauri::command]
pub async fn ingest_file(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Result<Document, String> {
    let path = PathBuf::from(path);
    let name = ingest::file_name(&path);
    let size = ingest::file_size(&path);
    let config = read(&state, settings::embed_config)?;
    let rag = read(&state, |connection| Ok(settings::rag_config(connection)))?;

    let progress = |stage: &str, done: usize, total: usize| {
        let _ = app.emit(
            "ingest-progress",
            IngestEvent {
                name: name.clone(),
                stage: stage.to_string(),
                done,
                total,
            },
        );
    };

    progress("lendo", 0, 0);
    let file = path.clone();
    let text = tauri::async_runtime::spawn_blocking(move || ingest::extract(&file))
        .await
        .map_err(|_| "A leitura do arquivo falhou.".to_string())??;

    let pieces = chunk::split(&text, rag.chunk_size, rag.chunk_overlap);
    if pieces.is_empty() {
        return Err("Não encontrei texto para indexar nesse arquivo.".to_string());
    }

    let mut vectors: Vec<Vec<f32>> = Vec::with_capacity(pieces.len());
    for batch in pieces.chunks(embed::BATCH) {
        progress("indexando", vectors.len(), pieces.len());
        vectors.extend(embed::embed_many(&state.http, &config, batch).await?);
    }
    progress("salvando", pieces.len(), pieces.len());

    let tag = config.tag();
    let document = write(&state, |connection| {
        let transaction = connection.transaction().map_err(|e| e.to_string())?;

        transaction
            .execute(
                "INSERT INTO documents (name, source_path, size_bytes, embed_model, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![
                    name,
                    path.to_string_lossy(),
                    size,
                    tag,
                    db::now_ms()
                ],
            )
            .map_err(|e| e.to_string())?;

        let document_id = transaction.last_insert_rowid();

        for (position, (content, vector)) in pieces.iter().zip(vectors.iter()).enumerate() {
            transaction
                .execute(
                    "INSERT INTO chunks (document_id, position, content, embedding, model)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    rusqlite::params![
                        document_id,
                        position as i64,
                        content,
                        db::embedding_to_blob(vector),
                        tag
                    ],
                )
                .map_err(|e| e.to_string())?;
        }

        transaction.commit().map_err(|e| e.to_string())?;

        db::list_documents(connection)?
            .into_iter()
            .find(|document| document.id == document_id)
            .ok_or_else(|| "Documento salvo, mas não foi possível recarregar.".to_string())
    })?;

    progress("pronto", pieces.len(), pieces.len());
    Ok(document)
}
