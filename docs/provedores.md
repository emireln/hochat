# Provedores

## Os seis

| Provedor | URL base | Modelo padrão | Formato da API |
| --- | --- | --- | --- |
| OpenAI | `https://api.openai.com/v1` | `gpt-5.6-terra` | OpenAI |
| Claude | `https://api.anthropic.com/v1` | `claude-sonnet-5` | Anthropic |
| Gemini | `https://generativelanguage.googleapis.com/v1beta` | `gemini-3.7-flash` | Google |
| DeepSeek | `https://api.deepseek.com/v1` | `deepseek-v4-flash` | OpenAI |
| GLM | `https://open.bigmodel.cn/api/paas/v4` | `glm-5.3` | OpenAI |
| Ollama | `http://127.0.0.1:11434` | `qwen3` | Ollama |

Três formatos, não seis. DeepSeek e GLM copiaram a API da OpenAI, então os três compartilham
`openai_compat.rs`. Claude, Gemini e Ollama têm cada um o seu arquivo.

A lista fica em `settings.rs`, e é literalmente uma constante:

```rust
pub const PROVIDERS: &[ProviderMeta] = &[
    ProviderMeta {
        id: "openai",
        label: "OpenAI",
        base_url: "https://api.openai.com/v1",
        default_model: "gpt-5.6-terra",
        needs_key: true,
        key_url: "https://platform.openai.com/api-keys",
        fallback_models: &["gpt-5.6-sol", "gpt-5.6-terra", "gpt-5.6-luna", "gpt-5.5", "gpt-5.4-mini"],
        supports_sampling: false,
        max_tokens_field: "max_completion_tokens",
    },
    // ...
];
```

Para adicionar um provedor com API compatível com OpenAI, basta mais uma entrada nesse array.
Se a API tiver formato próprio, aí precisa de um arquivo em `llm/` e de mais um braço no `match` de
`llm/mod.rs`.

## Nomes de modelo envelhecem rápido

Os padrões acima valem para setembro de 2026 e vão envelhecer. Dois casos que já me pegaram:

- **DeepSeek** aposentou `deepseek-chat` e `deepseek-reasoner` em 24 de julho de 2026. Não viraram
  apelido de nada: quem manda esses nomes hoje recebe erro. Os nomes atuais são `deepseek-v4-flash`
  e `deepseek-v4-pro`.
- **Gemini** deprecou toda a linha `gemini-1.5-*` e `gemini-2.0-*`.

Por isso o botão de recarregar ao lado do campo de modelo existe: ele pergunta ao provedor quais
modelos a sua chave enxerga de verdade. A lista fixa do código é só o que aparece quando não dá
para perguntar (sem chave, sem internet, provedor fora do ar), e nesse caso o app avisa que a
lista não é a real.

## Duas diferenças que o código precisa saber

`ProviderMeta` carrega dois campos que existem só por causa da OpenAI:

**`supports_sampling`** — a família GPT-5 aceita apenas a temperatura padrão e devolve erro se você
mandar outra. Então para a OpenAI o app simplesmente não envia `temperature` nem `top_p`, e a tela
de Configurações mostra um aviso explicando que os dois controles não valem para esse provedor.

**`max_tokens_field`** — a OpenAI trocou `max_tokens` por `max_completion_tokens` nos modelos novos.
Os outros continuam em `max_tokens`.

Preferi deixar essas duas diferenças explícitas numa constante a espalhar `if provider == "openai"`
pelos adapters.

## Configurando

Configurações → escolha o provedor no dropdown → cole a chave. Salva sozinho, sem botão de
confirmar.

O campo mostra `••••••••a1b2` quando já existe chave guardada. Digitar por cima substitui, e o
botão da lixeira apaga. A chave completa nunca é devolvida para a tela: `get_settings` só manda
`hasKey` e a máscara.

**Modelo** é um dropdown com busca que também aceita nome digitado — útil para modelo que acabou de
sair e ainda não está na lista. Cada provedor lembra o modelo dele, então trocar de OpenAI para
Claude e voltar não perde a escolha.

**URL base** existe para proxy, gateway corporativo, LiteLLM, ou Ollama em outra máquina. Deixar
vazio volta para o padrão.

**Testar conexão** faz a chamada mais barata que existe (listar modelos) e diz o que aconteceu.

Dá para trocar de modelo sem abrir Configurações: o seletor no topo da conversa lista os modelos de
todos os provedores que já têm chave, e escolher um troca provedor e modelo de uma vez.

## O que cada adapter faz

**OpenAI, DeepSeek e GLM** — `POST /chat/completions` com `stream: true`. Resposta em SSE, cada
linha `data:` traz um JSON e o texto está em `choices[0].delta.content`. Termina com
`data: [DONE]`. Autenticação por `Authorization: Bearer`.

**Claude** — `POST /messages`, cabeçalhos `x-api-key` e `anthropic-version: 2023-06-01`. A
instrução do sistema vai num campo `system` separado, não como mensagem. O stream manda eventos
com tipo; o que interessa é `content_block_delta`, e `message_stop` encerra. `max_tokens` é
obrigatório. A temperatura é limitada a 1.0 porque a API não aceita mais que isso.

**Gemini** — `POST /models/{modelo}:streamGenerateContent?alt=sse`, chave no cabeçalho
`x-goog-api-key`. Sem o `alt=sse` a resposta vem como um array JSON gigante no final, sem stream.
O papel do assistente chama `model` e não `assistant`, então há uma conversão. A instrução do
sistema vai em `systemInstruction` e os controles em `generationConfig`.

**Ollama** — `POST /api/chat`. Não é SSE: é uma linha de JSON por pedaço, e a última tem
`done: true`. Os controles vão em `options`, com `num_predict` no lugar de `max_tokens`. Modelos
vêm de `GET /api/tags`. Sem autenticação.

## Erros

O tratamento fica em `llm/stream.rs`, num lugar só. Ele lê o corpo do erro, procura a mensagem em
`error.message`, `error` ou `message` (cada API escolheu um) e prefixa uma explicação em português:

| Código | Vira |
| --- | --- |
| 401, 403 | API key inválida ou sem permissão |
| 404 | Modelo ou endpoint não encontrado |
| 429 | Limite de uso atingido |
| 5xx | O provedor está com problema |

Falha de conexão tem tratamento próprio, e no caso do Ollama a mensagem sugere `ollama serve`,
que é o motivo em nove de cada dez vezes.
