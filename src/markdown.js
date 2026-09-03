const ESCAPES = { "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" };

function escape(text) {
  return text.replace(/[&<>"]/g, (character) => ESCAPES[character]);
}

function inline(text) {
  const codes = [];
  let html = escape(text).replace(/`([^`]+)`/g, (_, code) => {
    codes.push(code);
    return `\u0000${codes.length - 1}\u0000`;
  });

  html = html
    .replace(/\[([^\]]+)\]\((https?:\/\/[^\s)]+)\)/g, '<a href="$2" data-external>$1</a>')
    .replace(/\*\*([^*]+)\*\*/g, "<strong>$1</strong>")
    .replace(/(^|[^*\w])\*([^*\n]+)\*/g, "$1<em>$2</em>");

  return html.replace(/\u0000(\d+)\u0000/g, (_, index) => `<code>${codes[index]}</code>`);
}

function closeList(state, output) {
  if (state.list) {
    output.push(`</${state.list}>`);
    state.list = null;
  }
}

export function render(source) {
  const lines = source.replace(/\r/g, "").split("\n");
  const output = [];
  const state = { list: null };
  let paragraph = [];
  let fence = null;

  const flush = () => {
    if (paragraph.length === 0) return;
    output.push(`<p>${inline(paragraph.join("\n"))}</p>`);
    paragraph = [];
  };

  for (const line of lines) {
    if (fence !== null) {
      if (line.trimEnd() === "```") {
        output.push(`<pre><code>${escape(fence.join("\n"))}</code></pre>`);
        fence = null;
      } else {
        fence.push(line);
      }
      continue;
    }

    if (line.trimStart().startsWith("```")) {
      flush();
      closeList(state, output);
      fence = [];
      continue;
    }

    if (line.trim() === "") {
      flush();
      closeList(state, output);
      continue;
    }

    const heading = line.match(/^(#{1,6})\s+(.*)$/);
    if (heading) {
      flush();
      closeList(state, output);
      output.push(`<h3>${inline(heading[2])}</h3>`);
      continue;
    }

    if (line.startsWith("> ")) {
      flush();
      closeList(state, output);
      output.push(`<blockquote>${inline(line.slice(2))}</blockquote>`);
      continue;
    }

    const bullet = line.match(/^\s*[-*]\s+(.*)$/);
    const numbered = line.match(/^\s*\d+[.)]\s+(.*)$/);

    if (bullet || numbered) {
      flush();
      const wanted = bullet ? "ul" : "ol";
      if (state.list !== wanted) {
        closeList(state, output);
        output.push(`<${wanted}>`);
        state.list = wanted;
      }
      output.push(`<li>${inline((bullet || numbered)[1])}</li>`);
      continue;
    }

    closeList(state, output);
    paragraph.push(line);
  }

  if (fence !== null) {
    output.push(`<pre><code>${escape(fence.join("\n"))}</code></pre>`);
  }
  flush();
  closeList(state, output);

  return output.join("");
}
