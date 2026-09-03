use crate::db;
use rusqlite::Connection;
use serde::Serialize;

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Hit {
    pub document: String,
    pub content: String,
    pub score: f32,
}

pub fn cosine(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let mut dot = 0.0;
    let mut norm_a = 0.0;
    let mut norm_b = 0.0;

    for (x, y) in a.iter().zip(b.iter()) {
        dot += x * y;
        norm_a += x * x;
        norm_b += y * y;
    }

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    dot / (norm_a.sqrt() * norm_b.sqrt())
}

pub fn top_k(
    conn: &Connection,
    query: &[f32],
    model_tag: &str,
    k: usize,
    min_score: f32,
) -> Result<Vec<Hit>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT d.name, c.content, c.embedding
             FROM chunks c JOIN documents d ON d.id = c.document_id
             WHERE c.model = ?1",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([model_tag], |row| {
            let embedding: Vec<u8> = row.get(2)?;
            Ok(Hit {
                document: row.get(0)?,
                content: row.get(1)?,
                score: cosine(query, &db::blob_to_embedding(&embedding)),
            })
        })
        .map_err(|e| e.to_string())?;

    let mut hits = rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    hits.sort_by(|a, b| b.score.total_cmp(&a.score));
    hits.truncate(k);
    hits.retain(|hit| hit.score > min_score);

    Ok(hits)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vetores_iguais_tem_score_um() {
        let vector = [0.2, 0.4, 0.6];
        assert!((cosine(&vector, &vector) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn vetores_ortogonais_tem_score_zero() {
        assert!(cosine(&[1.0, 0.0], &[0.0, 1.0]).abs() < 1e-6);
    }

    #[test]
    fn tamanhos_diferentes_nao_pontuam() {
        assert_eq!(cosine(&[1.0, 0.0], &[1.0]), 0.0);
    }
}
