pub fn split(text: &str, size: usize, overlap: usize) -> Vec<String> {
    let characters: Vec<char> = normalize(text).chars().collect();
    let size = size.max(200);
    let overlap = overlap.min(size / 2);

    let mut chunks = Vec::new();
    let mut start = 0;

    while start < characters.len() {
        let limit = (start + size).min(characters.len());
        let end = if limit == characters.len() {
            limit
        } else {
            boundary(&characters, start, limit)
        };

        let piece: String = characters[start..end].iter().collect();
        let piece = piece.trim().to_string();
        if !piece.is_empty() {
            chunks.push(piece);
        }

        if end >= characters.len() {
            break;
        }
        start = (end - overlap).max(start + 1);
    }

    chunks
}

fn boundary(characters: &[char], start: usize, limit: usize) -> usize {
    let floor = start + (limit - start) * 7 / 10;

    for index in (floor..limit).rev() {
        if characters[index] == '\n' {
            return index + 1;
        }
    }
    for index in (floor..limit).rev() {
        if matches!(characters[index], '.' | '!' | '?') {
            return index + 1;
        }
    }
    for index in (floor..limit).rev() {
        if characters[index] == ' ' {
            return index + 1;
        }
    }

    limit
}

fn normalize(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let mut blank_lines = 0;

    for line in text.replace('\r', "").lines() {
        let line = line.trim_end();
        if line.is_empty() {
            blank_lines += 1;
            if blank_lines > 1 {
                continue;
            }
        } else {
            blank_lines = 0;
        }
        output.push_str(line);
        output.push('\n');
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn respeita_tamanho_maximo() {
        let text = "palavra ".repeat(500);
        for chunk in split(&text, 400, 40) {
            assert!(chunk.chars().count() <= 400);
        }
    }

    #[test]
    fn mantem_acentos_intactos() {
        let text = "coração ".repeat(200);
        let chunks = split(&text, 300, 30);
        assert!(chunks.iter().all(|chunk| chunk.contains("coração")));
    }

    #[test]
    fn texto_curto_vira_um_chunk() {
        assert_eq!(split("resumo pequeno", 2000, 200).len(), 1);
    }
}
