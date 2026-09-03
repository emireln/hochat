use rusqlite::Connection;
use serde::Serialize;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const MIGRATIONS: &[&str] = &[include_str!("../migrations/001_inicial.sql")];

pub fn open(path: &Path) -> Result<Connection, String> {
    let conn = Connection::open(path).map_err(|e| e.to_string())?;
    conn.pragma_update(None, "journal_mode", "WAL")
        .map_err(|e| e.to_string())?;
    conn.pragma_update(None, "foreign_keys", "ON")
        .map_err(|e| e.to_string())?;
    migrate(&conn)?;
    Ok(conn)
}

pub fn migrate(conn: &Connection) -> Result<(), String> {
    let applied: i64 = conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(|e| e.to_string())?;

    for (index, sql) in MIGRATIONS.iter().enumerate().skip(applied as usize) {
        conn.execute_batch(sql).map_err(|e| e.to_string())?;
        conn.pragma_update(None, "user_version", (index + 1) as i64)
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

pub fn embedding_to_blob(vector: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(vector.len() * 4);
    for value in vector {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
}

pub fn blob_to_embedding(blob: &[u8]) -> Vec<f32> {
    blob.chunks_exact(4)
        .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .collect()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Chat {
    pub id: i64,
    pub title: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub id: i64,
    pub chat_id: i64,
    pub role: String,
    pub content: String,
    pub sources: Vec<String>,
    pub created_at: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Document {
    pub id: i64,
    pub name: String,
    pub source_path: String,
    pub size_bytes: i64,
    pub chunk_count: i64,
    pub embed_model: String,
    pub created_at: i64,
}

pub fn list_chats(conn: &Connection) -> Result<Vec<Chat>, String> {
    let mut stmt = conn
        .prepare("SELECT id, title, created_at, updated_at FROM chats ORDER BY updated_at DESC")
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([], |row| {
            Ok(Chat {
                id: row.get(0)?,
                title: row.get(1)?,
                created_at: row.get(2)?,
                updated_at: row.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?;

    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

pub fn create_chat(conn: &Connection, title: &str) -> Result<Chat, String> {
    let now = now_ms();
    conn.execute(
        "INSERT INTO chats (title, created_at, updated_at) VALUES (?1, ?2, ?2)",
        rusqlite::params![title, now],
    )
    .map_err(|e| e.to_string())?;

    Ok(Chat {
        id: conn.last_insert_rowid(),
        title: title.to_string(),
        created_at: now,
        updated_at: now,
    })
}

pub fn rename_chat(conn: &Connection, chat_id: i64, title: &str) -> Result<(), String> {
    conn.execute(
        "UPDATE chats SET title = ?1, updated_at = ?2 WHERE id = ?3",
        rusqlite::params![title, now_ms(), chat_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn delete_chat(conn: &Connection, chat_id: i64) -> Result<(), String> {
    conn.execute("DELETE FROM chats WHERE id = ?1", [chat_id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn chat_title(conn: &Connection, chat_id: i64) -> Result<String, String> {
    conn.query_row("SELECT title FROM chats WHERE id = ?1", [chat_id], |row| {
        row.get(0)
    })
    .map_err(|e| e.to_string())
}

pub fn list_messages(conn: &Connection, chat_id: i64) -> Result<Vec<Message>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, chat_id, role, content, sources, created_at
             FROM messages WHERE chat_id = ?1 ORDER BY id",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([chat_id], |row| {
            let sources: String = row.get(4)?;
            Ok(Message {
                id: row.get(0)?,
                chat_id: row.get(1)?,
                role: row.get(2)?,
                content: row.get(3)?,
                sources: serde_json::from_str(&sources).unwrap_or_default(),
                created_at: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?;

    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

pub fn insert_message(
    conn: &Connection,
    chat_id: i64,
    role: &str,
    content: &str,
    sources: &[String],
) -> Result<Message, String> {
    let now = now_ms();
    let sources_json = serde_json::to_string(sources).unwrap_or_else(|_| "[]".into());

    conn.execute(
        "INSERT INTO messages (chat_id, role, content, sources, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![chat_id, role, content, sources_json, now],
    )
    .map_err(|e| e.to_string())?;

    conn.execute(
        "UPDATE chats SET updated_at = ?1 WHERE id = ?2",
        rusqlite::params![now, chat_id],
    )
    .map_err(|e| e.to_string())?;

    Ok(Message {
        id: conn.last_insert_rowid(),
        chat_id,
        role: role.to_string(),
        content: content.to_string(),
        sources: sources.to_vec(),
        created_at: now,
    })
}

pub fn delete_message(conn: &Connection, message_id: i64) -> Result<(), String> {
    conn.execute("DELETE FROM messages WHERE id = ?1", [message_id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn list_documents(conn: &Connection) -> Result<Vec<Document>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT d.id, d.name, d.source_path, d.size_bytes, d.embed_model, d.created_at,
                    (SELECT COUNT(*) FROM chunks c WHERE c.document_id = d.id)
             FROM documents d ORDER BY d.created_at DESC",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([], |row| {
            Ok(Document {
                id: row.get(0)?,
                name: row.get(1)?,
                source_path: row.get(2)?,
                size_bytes: row.get(3)?,
                embed_model: row.get(4)?,
                created_at: row.get(5)?,
                chunk_count: row.get(6)?,
            })
        })
        .map_err(|e| e.to_string())?;

    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

pub fn delete_document(conn: &Connection, document_id: i64) -> Result<(), String> {
    conn.execute("DELETE FROM documents WHERE id = ?1", [document_id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn count_chunks(conn: &Connection, model_tag: &str) -> Result<i64, String> {
    conn.query_row(
        "SELECT COUNT(*) FROM chunks WHERE model = ?1",
        [model_tag],
        |row| row.get(0),
    )
    .map_err(|e| e.to_string())
}

#[cfg(test)]
pub fn memory() -> Connection {
    let conn = Connection::open_in_memory().expect("banco em memoria");
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    migrate(&conn).unwrap();
    conn
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conversa_guarda_mensagens_e_fontes() {
        let conn = memory();
        let chat = create_chat(&conn, "Nova conversa").unwrap();

        insert_message(&conn, chat.id, "user", "o que diz o manual?", &[]).unwrap();
        insert_message(
            &conn,
            chat.id,
            "assistant",
            "diz o seguinte",
            &["manual.pdf".to_string()],
        )
        .unwrap();

        let messages = list_messages(&conn, chat.id).unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, "user");
        assert_eq!(messages[1].sources, vec!["manual.pdf".to_string()]);
    }

    #[test]
    fn apagar_conversa_leva_as_mensagens_junto() {
        let conn = memory();
        let chat = create_chat(&conn, "Nova conversa").unwrap();
        insert_message(&conn, chat.id, "user", "oi", &[]).unwrap();

        delete_chat(&conn, chat.id).unwrap();

        assert!(list_messages(&conn, chat.id).unwrap().is_empty());
        assert!(list_chats(&conn).unwrap().is_empty());
    }

    #[test]
    fn embedding_sobrevive_a_ida_e_volta() {
        let vector = vec![0.5_f32, -1.25, 3.0, 0.0];
        assert_eq!(blob_to_embedding(&embedding_to_blob(&vector)), vector);
    }

    #[test]
    fn migracao_roda_uma_vez_so() {
        let conn = memory();
        migrate(&conn).unwrap();

        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, MIGRATIONS.len() as i64);
    }
}
