# HoChat

⚙️ EM DESENVOLVIMENTO

Chatbot de desktop com RAG local. Fiz para estudar RAG na prática, então tudo que dá para fazer
sem servidor roda aqui mesmo: conversas, documentos, embeddings e chaves ficam num SQLite na minha
máquina.

Backend em Rust (Tauri 2), frente em HTML, CSS e JavaScript puro. Sem framework, sem build
complicado.

- Conversa com OpenAI, Claude, Gemini, DeepSeek, GLM e modelos locais do Ollama
- Indexa `.txt`, `.md`, `.csv`, `.json` e `.pdf` e responde usando esses trechos
- Resposta em streaming, com botão de parar
- Tema claro e escuro, barra lateral que recolhe, atalhos de teclado
- Instalador de 3,3 MB; a interface inteira dá 68 KB de JS, CSS e fonte de ícones

| Atalho | O que faz |
| --- | --- |
| `Ctrl+B` | recolhe ou mostra a barra lateral |
| `Ctrl+N` | conversa nova |
| `Ctrl+,` | configurações |
| `Esc` | volta para a conversa |

## Rodando

```bash
npm install
npm run app
```

Precisa de Rust e Node. O passo a passo completo está em [docs/setup.md](docs/setup.md).

## Documentação

| Arquivo | Assunto |
| --- | --- |
| [docs/visao.md](docs/visao.md) | O que o HoChat é e o que decidi não fazer |
| [docs/setup.md](docs/setup.md) | Instalar, rodar em dev e gerar o instalador |
| [docs/arquitetura.md](docs/arquitetura.md) | Pastas, banco, comandos e o caminho de uma mensagem |
| [docs/provedores.md](docs/provedores.md) | Chaves, URLs base e como cada API é falada |
| [docs/rag.md](docs/rag.md) | Chunking, embeddings, busca por cosseno e limites |
| [docs/ui.md](docs/ui.md) | Paleta, ícones, animações e as três telas |
