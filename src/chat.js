import { confirm } from "@tauri-apps/plugin-dialog";
import { api, listen } from "./api.js";
import { createDropdown } from "./dropdown.js";
import { render } from "./markdown.js";
import { currentSettings } from "./settings.js";
import { autosize, element, icon, toast } from "./ui.js";
import { setChatTitle, showView } from "./views.js";
import logo from "./assets/hochat-logo.svg";
import { getProviderLogo } from "./assets/providers.js";

const list = document.getElementById("chat-list");
const container = document.getElementById("messages");
const composer = document.getElementById("composer");
const prompt = document.getElementById("prompt");
const sendButton = document.getElementById("send");
const ragToggle = document.getElementById("rag-toggle");
const pickerSlot = document.getElementById("model-picker");

const state = {
  chats: [],
  activeId: null,
  messages: [],
  useRag: localStorage.getItem("hochat.rag") === "on",
  streams: new Map(),
  settings: null,
  liveModels: new Map(),
};

let picker = null;
let frame = 0;

export async function initChat() {
  ragToggle.setAttribute("aria-pressed", String(state.useRag));

  picker = createDropdown({
    compact: true,
    searchable: true,
    placeholder: "Escolher modelo",
    onChange: onPickModel,
  });
  pickerSlot.append(picker.element);

  composer.addEventListener("submit", onSubmit);
  prompt.addEventListener("input", () => autosize(prompt));
  prompt.addEventListener("keydown", onKeydown);
  ragToggle.addEventListener("click", toggleRag);
  document.getElementById("new-chat").addEventListener("click", () => newChat());
  document.getElementById("delete-chat").addEventListener("click", removeActive);
  document.addEventListener("settings-changed", (event) => onSettings(event.detail));
  document.addEventListener("chats-cleared", () => reload());

  await listen("chat-token", ({ payload }) => onToken(payload));
  await listen("chat-sources", ({ payload }) => onSources(payload));
  await listen("chat-title", ({ payload }) => onTitle(payload));

  const settings = currentSettings();
  if (settings) onSettings(settings);

  await reload();
}

async function reload() {
  state.chats = await api.listChats();
  state.streams.clear();
  renderList();

  if (state.chats.length > 0) await open(state.chats[0].id);
  else await newChat();
}

function usableProviders() {
  return state.settings.providers.filter((item) => !item.needsKey || item.hasKey);
}

function onSettings(settings) {
  const previous = state.settings?.provider;
  state.settings = settings;

  paintPicker();
  paintComposerHint();

  if (settings.provider !== previous) fetchLiveModels(settings.provider);
}

function paintPicker() {
  const settings = state.settings;
  if (!settings) return;

  const items = [];

  for (const provider of usableProviders()) {
    const names = state.liveModels.get(provider.id) ?? provider.models;
    const all = names.includes(provider.model) ? names : [provider.model, ...names];

    for (const name of all) {
      items.push({
        value: `${provider.id}::${name}`,
        label: name,
        hint: provider.label,
        logoSrc: getProviderLogo(provider.id),
      });
    }
  }

  const active = settings.providers.find((item) => item.id === settings.provider);
  picker.setItems(items);
  picker.setValue(active ? `${active.id}::${active.model}` : null);
}

async function fetchLiveModels(providerId) {
  try {
    const result = await api.listModels(providerId);
    if (!result.live) return;
    state.liveModels.set(providerId, result.models);
    paintPicker();
  } catch {
    /* a lista estatica ja cobre esse caso */
  }
}

function onPickModel(choice) {
  const [provider, model] = choice.split("::");
  document.dispatchEvent(
    new CustomEvent("model-picked", { detail: { provider, model } })
  );
}

function paintComposerHint() {
  prompt.placeholder = state.settings.sendOnEnter
    ? "Pergunte alguma coisa. Enter envia, Shift+Enter quebra linha."
    : "Pergunte alguma coisa. Ctrl+Enter envia.";
}

function renderList() {
  if (state.chats.length === 0) {
    list.replaceChildren(element("p", "sidebar-empty", "Nenhuma conversa ainda."));
    return;
  }

  const rows = state.chats.map((chat) => {
    const row = element("div", "chat-row");
    if (chat.id === state.activeId) row.classList.add("active");

    const label = element("button", "chat-row-title", chat.title);
    label.type = "button";
    label.title = chat.title;
    label.addEventListener("click", () => open(chat.id));

    const remove = element("button", "chat-row-remove");
    remove.type = "button";
    remove.title = "Apagar conversa";
    remove.append(icon("close"));
    remove.addEventListener("click", (event) => {
      event.stopPropagation();
      removeChat(chat.id);
    });

    row.append(label, remove);
    return row;
  });

  list.replaceChildren(...rows);
}

async function newChat() {
  const pending = state.chats.find((chat) => chat.title === "Nova conversa");
  if (pending) {
    await open(pending.id);
    prompt.focus();
    return;
  }

  try {
    const chat = await api.createChat();
    state.chats.unshift(chat);
    await open(chat.id);
    prompt.focus();
  } catch (error) {
    toast(error, "error");
  }
}

async function open(chatId) {
  state.activeId = chatId;
  const chat = state.chats.find((item) => item.id === chatId);
  setChatTitle(chat ? chat.title : "Conversa");
  showView("chat");
  renderList();

  try {
    state.messages = await api.listMessages(chatId);
  } catch (error) {
    state.messages = [];
    toast(error, "error");
  }

  renderMessages();
  setSending(state.streams.has(chatId));
}

async function removeChat(chatId) {
  const chat = state.chats.find((item) => item.id === chatId);
  const empty = chat && chat.title === "Nova conversa";

  if (!empty) {
    const confirmed = await confirm("Apagar esta conversa e todas as mensagens dela?", {
      title: "HoChat",
      kind: "warning",
      okLabel: "Apagar",
      cancelLabel: "Cancelar",
    });
    if (!confirmed) return;
  }

  try {
    await api.deleteChat(chatId);
    state.chats = state.chats.filter((item) => item.id !== chatId);
    state.streams.delete(chatId);

    if (state.activeId === chatId) {
      if (state.chats.length > 0) await open(state.chats[0].id);
      else await newChat();
    } else {
      renderList();
    }
  } catch (error) {
    toast(error, "error");
  }
}

function removeActive() {
  if (state.activeId !== null) removeChat(state.activeId);
}

function renderMessages() {
  const inner = element("div", "messages-inner");

  if (state.messages.length === 0 && !state.streams.has(state.activeId)) {
    inner.append(emptyState());
  } else {
    for (const message of state.messages) inner.append(messageNode(message));
  }

  const stream = state.streams.get(state.activeId);
  if (stream) inner.append(streamNode(stream));

  container.replaceChildren(inner);
  scrollToEnd(true);
}

function emptyState() {
  const box = element("div", "empty-chat");
  const mark = document.createElement("img");
  mark.src = logo;
  mark.alt = "";

  box.append(
    mark,
    element("strong", null, "Comece uma conversa"),
    element(
      "p",
      null,
      "Escolha o modelo aqui em cima e pergunte. Ligue os documentos para responder com a sua base."
    )
  );
  return box;
}

function messageNode(message) {
  if (message.role === "user") {
    const node = element("div", "message user");
    node.append(element("div", "message-author", "Você"));
    node.append(element("div", "bubble", message.content));
    return node;
  }

  const node = element("div", "message");
  node.append(author());

  const answer = element("div", "answer");
  answer.innerHTML = render(message.content);
  node.append(answer);

  if (message.sources && message.sources.length > 0) {
    node.append(sourcesNode(message.sources));
  }
  return node;
}

function author() {
  const box = element("div", "message-author");
  const mark = document.createElement("img");
  mark.src = logo;
  mark.alt = "";
  box.append(mark, element("span", null, "HoChat"));
  return box;
}

function sourcesNode(sources) {
  const box = element("div", "sources");
  for (const name of sources) {
    const tag = element("span", "source-tag");
    tag.append(icon("description"), element("span", null, name));
    box.append(tag);
  }
  return box;
}

function streamNode(stream) {
  const node = element("div", "message");
  node.append(author());

  const answer = element("div", "answer streaming");
  answer.textContent = stream.buffer;
  stream.node = answer;
  node.append(answer);

  if (stream.sources.length > 0) node.append(sourcesNode(stream.sources));
  return node;
}

function onKeydown(event) {
  if (event.key !== "Enter" || event.isComposing) return;

  const sendOnEnter = state.settings?.sendOnEnter ?? true;
  const shouldSend = sendOnEnter ? !event.shiftKey : event.ctrlKey;

  if (shouldSend) {
    event.preventDefault();
    composer.requestSubmit();
  }
}

function toggleRag() {
  state.useRag = !state.useRag;
  ragToggle.setAttribute("aria-pressed", String(state.useRag));
  localStorage.setItem("hochat.rag", state.useRag ? "on" : "off");
}

async function onSubmit(event) {
  event.preventDefault();

  const chatId = state.activeId;
  if (chatId === null) return;

  if (state.streams.has(chatId)) {
    api.stopGeneration(chatId);
    return;
  }

  const content = prompt.value.trim();
  if (content === "") return;

  prompt.value = "";
  autosize(prompt);
  setSending(true);

  state.messages.push({ id: -Date.now(), role: "user", content, sources: [] });
  state.streams.set(chatId, { buffer: "", sources: [], node: null });
  renderMessages();

  try {
    const answer = await api.sendMessage(chatId, content, state.useRag);
    state.streams.delete(chatId);

    if (chatId === state.activeId) {
      state.messages = await api.listMessages(chatId);
      renderMessages();
    }
    touch(chatId, answer.createdAt);
  } catch (error) {
    state.streams.delete(chatId);

    if (chatId === state.activeId) {
      state.messages.pop();
      renderMessages();
      container.querySelector(".messages-inner")?.append(errorNode(error));
      scrollToEnd();
      prompt.value = content;
      autosize(prompt);
    } else {
      toast(error, "error");
    }
  } finally {
    if (chatId === state.activeId) setSending(false);
  }
}

function errorNode(error) {
  const box = element("div", "message-error");
  box.append(icon("error"), element("span", null, String(error)));
  return box;
}

function setSending(sending) {
  sendButton.classList.toggle("stop", sending);
  sendButton.title = sending ? "Parar" : "Enviar";
  sendButton.replaceChildren(icon(sending ? "stop" : "send"));
}

function touch(chatId, updatedAt) {
  const index = state.chats.findIndex((chat) => chat.id === chatId);
  if (index < 0) return;

  const [chat] = state.chats.splice(index, 1);
  chat.updatedAt = updatedAt;
  state.chats.unshift(chat);
  renderList();
}

function onToken({ chatId, text }) {
  const stream = state.streams.get(chatId);
  if (!stream) return;

  stream.buffer += text;
  if (chatId !== state.activeId || !stream.node || frame) return;

  frame = requestAnimationFrame(() => {
    frame = 0;
    const current = state.streams.get(state.activeId);
    if (!current || !current.node) return;
    current.node.textContent = current.buffer;
    scrollToEnd();
  });
}

function onSources({ chatId, hits }) {
  const stream = state.streams.get(chatId);
  if (!stream) return;

  stream.sources = [...new Set(hits.map((hit) => hit.document))];
  if (chatId === state.activeId) renderMessages();
}

function onTitle({ chatId, title }) {
  const chat = state.chats.find((item) => item.id === chatId);
  if (chat) chat.title = title;
  if (chatId === state.activeId) setChatTitle(title);
  renderList();
}

function scrollToEnd(force = false) {
  const distance = container.scrollHeight - container.scrollTop - container.clientHeight;
  if (force || distance < 220) container.scrollTop = container.scrollHeight;
}
