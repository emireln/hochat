# Interface

## O que eu queria evitar

Interface de app de IA hoje tem um visual padrão: gradiente roxo, vidro fosco, brilho atrás dos
botões, borda que muda de cor. Fica todo mundo igual e nada disso ajuda a ler texto, que é 90% do
que se faz aqui.

## Nenhuma linha divisória

Essa é a regra que mais mudou as coisas. Não existe uma `border` de separação em lugar nenhum do
app. Nem entre a barra lateral e a conversa, nem embaixo da barra de título, nem em volta de campo
ou de cartão.

Quando você tira as linhas precisa separar as coisas de outro jeito, e sobram três ferramentas:

**Cor de fundo.** A barra lateral usa `--bg`, a área principal usa `--surface`. São dois cinzas
quase iguais, e é esse degrau que marca onde uma termina e a outra começa. Não precisa de linha
para o olho entender.

**Preenchimento.** Cartão, campo de texto e botão secundário usam `--fill`, um tom entre os dois.
Um campo é uma superfície um pouco diferente do que está atrás, não um retângulo contornado.

**Espaço.** Onde antes tinha uma linha agora tem 16 ou 24px de respiro.

A única coisa que ainda desenha contorno é o foco do teclado, que é `box-shadow` de 2px na cor de
destaque. Isso é acessibilidade, não decoração.

## Cores

Seis tokens fazem o app inteiro, e o tema escuro só troca os valores:

| Token | Claro | Escuro | Onde |
| --- | --- | --- | --- |
| `--bg` | `#f4f4f5` | `#121215` | barra lateral |
| `--surface` | `#ffffff` | `#1a1a1f` | área principal, campos, menus |
| `--fill` | `#f1f1f3` | `#26262d` | cartões, botões, chips |
| `--text` | `#17171a` | `#f2f2f5` | texto |
| `--text-soft` | `#6c6c74` | `#a0a0aa` | rótulo, legenda |
| `--accent` | `#1d4ed8` | `#7aa2ff` | o brilho do logo |

O destaque no tema claro é o mesmo azul do logo. No escuro ele clareia, senão some no fundo.

O tema tem três opções em Configurações: claro, escuro e seguir o sistema. A escolha vai para o
SQLite, o JavaScript troca o `data-theme` no `<html>` e o CSS resolve o resto. Quando está em
"seguir o sistema", um `matchMedia` reage à troca sem precisar reiniciar. O app também avisa a
janela nativa com `setTheme`, senão a barra de título fica clara com o app escuro.

## Tipografia e ícones

Fonte do sistema: `Segoe UI` no Windows, `system-ui` no resto. Nada para baixar, nada para esperar.

Ícones são **Material Symbols Outlined**, hospedados junto com o app em `src/assets/`. Não uso o
CDN do Google: seria uma requisição externa toda vez que o app abre, num app que se propõe a
funcionar offline.

O arquivo tem 4 KB. A fonte completa tem uns 3,5 MB, mas o Google Fonts aceita o parâmetro
`icon_names` e devolve só os glifos pedidos. São 28 aqui. Para adicionar um ícone novo é preciso
baixar de novo com a lista atualizada:

```
https://fonts.googleapis.com/css2?family=Material+Symbols+Outlined:opsz,wght,FILL,GRAD@24,400,0,0&icon_names=add,arrow_back,...
```

## O layout

```
+-- barra lateral --+-- área principal --------------------+
| logo  HoChat      | [<] Nova conversa      [modelo v] 🗑  |
| + Nova conversa   |                                      |
|                   | mensagens                            |
| conversa 1        |                                      |
| conversa 2        |                                      |
|                   |                                      |
| ----------------- | +----------------------------------+ |
| Base de conhec. 3 | | campo de digitação                | |
| Configurações     | | [documentos]             [enviar] | |
+-------------------+ +----------------------------------+ |
```

A barra de título é uma só, no topo da área principal, e vale para as três telas. Ela tem sempre a
mesma estrutura: botão de recolher, título, ações da tela atual. O que muda entre telas é só o
grupo de ações da direita — seletor de modelo no chat, botão de adicionar arquivo na base, aviso de
"Salvo" nas configurações.

Antes cada tela tinha o seu próprio cabeçalho com tamanhos e espaçamentos ligeiramente diferentes.
Uma barra só resolveu isso e ainda deixou o código menor.

## Recolher a barra lateral

Botão no canto superior esquerdo ou `Ctrl+B`. O estado fica no `localStorage`, então continua como
você deixou na próxima vez que abrir.

A barra desliza com `margin-left: -248px` em vez de encolher a largura. A diferença importa: com
`width` animado o conteúdo dela vai sendo espremido durante os 220ms e o texto quebra em lugares
estranhos no meio do caminho. Com margem negativa ela mantém os 248px e simplesmente sai da tela.

## O padrão de dropdown

Todo seletor do app é o mesmo componente, em `src/dropdown.js`. Não sobrou nenhum `<select>`
nativo, que além de feio não deixa estilizar a lista aberta.

```js
createDropdown({
  items: [{ value, label, hint }],
  searchable: true,
  allowCustom: true,
  compact: false,
  onChange: (value) => {},
});
```

Ele é usado no provedor, no modelo, no provedor e modelo de embedding, no tema, na tecla de enviar
e no seletor de modelo do topo. Quatro detalhes que fazem ele funcionar:

**O menu vive no `<body>`.** Com posição `fixed` calculada a partir do botão. Se ficasse dentro do
cartão, o `overflow-y: auto` da tela de configurações cortaria a lista aberta. Ao rolar ou
redimensionar, o menu fecha em vez de tentar acompanhar.

**Ele vira para cima quando não cabe embaixo.** Compara o espaço disponível antes de escolher o
lado.

**Busca e valor livre.** Com `allowCustom`, o que você digitar vira a primeira opção da lista como
"usar assim". É como se digita um modelo que acabou de ser lançado e ainda não está em lista
nenhuma.

**Teclado.** Setas movem, Enter escolhe, Esc fecha e devolve o foco ao botão.

## Atalhos

| Tecla | O que faz |
| --- | --- |
| `Ctrl+B` | recolhe ou mostra a barra lateral |
| `Ctrl+N` | conversa nova |
| `Ctrl+,` | abre as configurações |
| `Esc` | volta para a conversa |
| `Enter` | envia (ou quebra linha, se você inverter em Configurações) |

## Animação

Regra única: só `transform` e `opacity`. São as duas propriedades que o navegador anima na GPU sem
recalcular layout. Animar `height`, `top` ou `width` força reflow e é onde o travamento aparece.

Duração entre 140ms e 220ms. Abaixo de 150ms parece falha de renderização; acima de 300ms parece
que o app está devendo resposta.

| Onde | O que |
| --- | --- |
| Mensagem nova | sobe 8px e aparece |
| Troca de tela | sobe 6px e aparece |
| Item de lista | sobe 4px e aparece |
| Menu de dropdown | escala de 0.98 e aparece, ancorado no topo |
| Botão de enviar | encolhe para 94% ao clicar |
| Cursor durante o stream | pisca em 2 passos |

Nada de biblioteca de animação. É tudo `@keyframes` e `transition`.

`prefers-reduced-motion` derruba tudo para 1ms.

## Por que não trava

Três decisões:

**O stream escreve texto puro.** Durante a resposta o conteúdo vai para `textContent`, que não
parseia nada. Markdown só roda uma vez, quando a resposta termina.

**Um repaint por frame.** Os tokens chegam mais rápido que 60 por segundo. Em vez de escrever na
tela a cada evento, eles se acumulam num buffer e um `requestAnimationFrame` escreve o acumulado.
Chegando 5 tokens no mesmo frame, é um repaint em vez de cinco.

**Rolagem que respeita quem está lendo.** A tela só desce sozinha se você já estiver a menos de
220px do fim. Se subiu para reler alguma coisa, ela fica parada.

## Sobre o markdown

`markdown.js` faz negrito, itálico, código, bloco de código, título, lista, citação e link. Nada
além disso.

Escrevi em vez de usar `marked` por dois motivos: são 90 linhas contra uma dependência, e escapar o
HTML antes de qualquer coisa deixa o caminho de injeção fechado por construção. O texto vem de um
modelo de linguagem — não é código malicioso, mas também não é conteúdo que eu controlo.

Link não navega dentro do app. O clique é interceptado e abre no navegador do sistema.
