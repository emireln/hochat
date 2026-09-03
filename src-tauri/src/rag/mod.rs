pub mod chunk;
pub mod embed;
pub mod ingest;
pub mod search;

pub fn build_context(hits: &[search::Hit]) -> String {
    let mut context = String::from(
        "Use os trechos abaixo para responder. Cite o documento quando usar um trecho. \
         Se a resposta não estiver neles, diga que não encontrou nos documentos.\n\n",
    );

    for (index, hit) in hits.iter().enumerate() {
        context.push_str(&format!(
            "[{}] {}\n{}\n\n",
            index + 1,
            hit.document,
            hit.content
        ));
    }

    context
}
