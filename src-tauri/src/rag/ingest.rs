use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::Path;

pub const SUPPORTED: &[&str] = &["txt", "md", "markdown", "csv", "json", "pdf"];

pub fn extract(path: &Path) -> Result<String, String> {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_lowercase();

    let text = match extension.as_str() {
        "pdf" => from_pdf(path)?,
        "txt" | "md" | "markdown" | "csv" | "json" => from_plain_text(path)?,
        other => {
            return Err(format!(
                "Formato .{other} não suportado. Use: {}.",
                SUPPORTED.join(", ")
            ))
        }
    };

    if text.trim().is_empty() {
        return Err("O arquivo não tem texto extraível.".to_string());
    }

    Ok(text)
}

fn from_plain_text(path: &Path) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("Não consegui ler o arquivo: {e}"))?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

fn from_pdf(path: &Path) -> Result<String, String> {
    let owned = path.to_path_buf();

    catch_unwind(AssertUnwindSafe(|| pdf_extract::extract_text(&owned)))
        .map_err(|_| "Não consegui ler esse PDF. Ele pode estar protegido ou corrompido.".to_string())?
        .map_err(|e| format!("Falha ao extrair o texto do PDF: {e}"))
}

pub fn file_name(path: &Path) -> String {
    path.file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("documento")
        .to_string()
}

pub fn file_size(path: &Path) -> i64 {
    std::fs::metadata(path)
        .map(|data| data.len() as i64)
        .unwrap_or(0)
}
