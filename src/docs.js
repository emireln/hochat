import { getCurrentWebview } from "@tauri-apps/api/webview";
import { confirm, open } from "@tauri-apps/plugin-dialog";
import { api, listen } from "./api.js";
import { element, formatBytes, formatDate, icon, stat, toast } from "./ui.js";

const notice = document.getElementById("rag-notice");
const summary = document.getElementById("rag-summary");
const list = document.getElementById("doc-list");
const addButton = document.getElementById("add-document");
const counter = document.getElementById("doc-count");
const veil = document.getElementById("drop-veil");

const state = {
  documents: [],
  status: null,
  pending: new Map(),
};

export async function initDocs() {
  addButton.addEventListener("click", pick);

  await listen("ingest-progress", ({ payload }) => {
    if (payload.stage === "pronto") return;
    state.pending.set(payload.name, payload);
    renderList();
  });

  await getCurrentWebview().onDragDropEvent(({ payload }) => {
    if (payload.type === "over" || payload.type === "enter") veil.hidden = false;
    else veil.hidden = true;

    if (payload.type === "drop") ingest(payload.paths);
  });

  await refreshDocs();
}

export async function refreshDocs() {
  try {
    const [status, documents] = await Promise.all([api.ragStatus(), api.listDocuments()]);
    state.status = status;
    state.documents = documents;
  } catch (error) {
    toast(error, "error");
    return;
  }

  counter.textContent = state.documents.length > 0 ? String(state.documents.length) : "";
  renderStatus();
  renderList();
}

function renderStatus() {
  const status = state.status;

  if (status.ready) {
    notice.hidden = true;
  } else {
    notice.hidden = false;
    notice.replaceChildren(
      icon("error"),
      element(
        "span",
        null,
        `${status.message} Sem embeddings o app continua conversando, mas não consulta os documentos.`
      )
    );
  }

  summary.replaceChildren(
    stat("Documentos", String(status.documents)),
    stat("Trechos indexados", String(status.chunks)),
    stat("Modelo de embedding", status.ready ? `${status.provider} / ${status.model}` : "-"),
    stat("Formatos aceitos", status.supported.join(", "))
  );
}

function renderList() {
  const rows = [];

  for (const item of state.pending.values()) {
    const row = element("div", "doc-row pending");
    row.append(icon("refresh"));

    const info = element("div", "doc-info");
    info.append(
      element("div", "doc-name", item.name),
      element(
        "div",
        "doc-meta",
        item.total > 0 ? `${item.stage} ${item.done}/${item.total} trechos` : item.stage
      )
    );
    row.append(info);
    rows.push(row);
  }

  for (const document of state.documents) {
    rows.push(documentRow(document));
  }

  if (rows.length === 0) {
    list.replaceChildren(
      element(
        "p",
        "empty-state",
        "Nenhum documento ainda. Adicione um arquivo ou arraste ele para a janela."
      )
    );
    return;
  }

  list.replaceChildren(...rows);
}

function documentRow(document) {
  const row = element("div", "doc-row");
  row.append(icon("description"));

  const info = element("div", "doc-info");
  info.append(
    element("div", "doc-name", document.name),
    element(
      "div",
      "doc-meta",
      `${document.chunkCount} trechos - ${formatBytes(document.sizeBytes)} - ${formatDate(
        document.createdAt
      )} - ${document.embedModel}`
    )
  );

  const remove = element("button", "button ghost danger");
  remove.type = "button";
  remove.title = "Remover documento";
  remove.append(icon("delete"));
  remove.addEventListener("click", () => removeDocument(document));

  row.append(info, remove);
  return row;
}

async function pick() {
  const status = state.status;
  const selected = await open({
    multiple: true,
    filters: [{ name: "Documentos", extensions: status ? status.supported : ["txt", "md", "pdf"] }],
  });

  if (!selected) return;
  await ingest(Array.isArray(selected) ? selected : [selected]);
}

async function ingest(paths) {
  if (!state.status || !state.status.ready) {
    toast("Configure os embeddings antes de indexar documentos.", "error");
    return;
  }

  for (const path of paths) {
    const name = path.split(/[\\/]/).pop();
    state.pending.set(name, { name, stage: "lendo", done: 0, total: 0 });
    renderList();

    try {
      await api.ingestFile(path);
      toast(`${name} indexado.`);
    } catch (error) {
      toast(error, "error");
    } finally {
      state.pending.delete(name);
    }
  }

  await refreshDocs();
}

async function removeDocument(document) {
  const confirmed = await confirm(`Remover "${document.name}" da base?`, {
    title: "HoChat",
    kind: "warning",
    okLabel: "Remover",
    cancelLabel: "Cancelar",
  });
  if (!confirmed) return;

  try {
    await api.deleteDocument(document.id);
    await refreshDocs();
  } catch (error) {
    toast(error, "error");
  }
}
