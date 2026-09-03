# Setup

## O que precisa estar instalado

| Ferramenta | Versão | Para quê |
| --- | --- | --- |
| Node | 20 ou mais novo | roda o Vite e o CLI do Tauri |
| Rust | 1.77 ou mais novo | compila o backend |
| Build Tools do Visual Studio | com "Desktop development with C++" | o SQLite vem embutido e precisa de compilador C |
| WebView2 | já vem no Windows 11 | é o motor que desenha a interface |

No macOS troque o Build Tools por `xcode-select --install`. No Linux é preciso `libwebkit2gtk-4.1-dev`,
`build-essential`, `libssl-dev` e `librsvg2-dev`.

## Primeira vez

```bash
npm install
npm run app
```

O `npm run app` sobe o Vite na porta 5173 e compila o Rust. A primeira compilação demora uns
minutos porque baixa e compila umas 450 crates. Depois disso é questão de segundos.

Editar arquivo dentro de `src/` recarrega a janela na hora. Editar arquivo dentro de `src-tauri/`
recompila e reinicia o app.

## Scripts

| Comando | O que faz |
| --- | --- |
| `npm run app` | modo desenvolvimento, com recarga automática |
| `npm run app:build` | gera o `.msi` e o `.exe` de instalação |
| `npm run dev` | só o Vite, sem a janela nativa (útil para mexer só no CSS) |
| `npm run build` | só o bundle do front, em `dist/` |
| `cargo test` | roda os testes de chunking e de similaridade (dentro de `src-tauri/`) |

O instalador sai em `src-tauri/target/release/bundle/`: um `.exe` do NSIS com 3,3 MB e um `.msi`
com 4,5 MB. O executável instalado tem 11 MB, e boa parte disso é o SQLite embutido e o parser de
PDF.

## Ollama (opcional, mas é o caminho mais fácil)

O HoChat vem configurado para o Ollama porque não precisa de chave nem de cartão. Instale pelo
[ollama.com](https://ollama.com/download) e baixe dois modelos:

```bash
ollama pull qwen3             # conversa
ollama pull nomic-embed-text  # embeddings do RAG
```

Deixe o `ollama serve` rodando. O HoChat fala com ele em `http://127.0.0.1:11434`.

Se preferir usar uma API paga, abra **Configurações**, escolha o provedor, cole a chave e pronto.
Detalhes em [provedores.md](provedores.md).

## Onde ficam os dados

Tudo num arquivo só:

```
%APPDATA%\com.hochat.desktop\hochat.db
```

No macOS é `~/Library/Application Support/com.hochat.desktop/` e no Linux
`~/.local/share/com.hochat.desktop/`.

Apagar esse arquivo zera o app: conversas, documentos, embeddings e chaves. Não tem backup
automático, é de propósito.

## Quando dá problema

**"Ollama não respondeu"** — o `ollama serve` não está no ar, ou está em outra porta. Confira a URL
base em Configurações.

**Erro 404 ao indexar documento** — falta o modelo de embedding. Rode `ollama pull nomic-embed-text`.

**A janela abre em branco** — o Vite não subiu. Olhe o terminal: normalmente é a porta 5173 já
ocupada por outra coisa.

**`link.exe not found` na compilação** — falta o "Desktop development with C++" nos Build Tools do
Visual Studio.
