# Visão

## Por que existe

Eu queria entender RAG de verdade, não só ler sobre. A forma que funciona para mim é montar a coisa
inteira: ler um arquivo, cortar em pedaços, gerar embedding, guardar, buscar e enfiar o resultado no
prompt. Cada etapa aqui está escrita à mão, sem LangChain, sem framework de agente, sem banco
vetorial externo. Se algo dá errado, dá para abrir o SQLite e olhar.

O segundo motivo é prático: eu queria um chat que abre rápido, que funciona offline com Ollama e
onde as minhas chaves de API não passam por servidor de terceiro.

## O que ele faz

- Conversa com seis provedores (OpenAI, Claude, Gemini, DeepSeek, GLM e Ollama), trocando com um
  clique e mantendo o modelo escolhido de cada um.
- Recebe `.txt`, `.md`, `.csv`, `.json` e `.pdf`, corta em trechos, gera embeddings e guarda tudo
  no mesmo banco das conversas.
- Quando o botão "Usar documentos" está ligado, busca os trechos mais parecidos com a pergunta e
  manda junto no prompt, mostrando embaixo da resposta de quais arquivos ela veio.
- Faz stream da resposta token a token e deixa parar no meio, salvando o que já chegou.
- Guarda o histórico de conversas e deixa apagar o que não interessa mais.
- Deixa mexer no que importa: temperatura, top-p, teto de tokens, quanto de histórico reenviar,
  tamanho do trecho, sobreposição e nota mínima da busca.

## O que decidi não fazer

Isso aqui é um projeto de estudo, não um produto. Cortei de propósito:

- **Login, nuvem e sincronização.** Um banco local, um usuário. Nada de servidor.
- **Banco vetorial dedicado.** Qdrant e afins resolvem um problema de escala que eu não tenho.
  Com alguns milhares de trechos, comparar tudo em Rust custa poucos milissegundos.
- **Agentes e ferramentas.** Sem function calling, sem loop de tarefas. Só pergunta e resposta.
- **OCR.** PDF escaneado não vira texto aqui. Se o PDF não tem camada de texto, o app avisa.
- **Múltiplos idiomas na interface.** É em português e pronto.

## As regras que segui

Quatro coisas que usei para decidir quando estava na dúvida:

1. **Chave de API nunca sai do Rust.** O JavaScript pede "está configurado?" e recebe uma máscara
   tipo `••••••••a1b2`. A chave completa só existe no SQLite e na hora do request HTTP.
2. **Erro tem que explicar o que fazer.** "HTTP 401" não ajuda ninguém. O app traduz para
   "API key inválida ou sem permissão" e, no caso do Ollama, sugere `ollama serve`.
3. **Nada de dependência que eu não entenda.** A lista de crates e de pacotes npm cabe em meia tela.
4. **Sem lag.** Durante o stream a tela escreve texto puro, uma vez por frame. Markdown só é
   processado quando a resposta termina.
