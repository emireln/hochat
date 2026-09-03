import { confirm } from "@tauri-apps/plugin-dialog";
import { api } from "./api.js";
import { createDropdown } from "./dropdown.js";
import { element, formatBytes, icon, stat, toast } from "./ui.js";
import { getProviderLogo } from "./assets/providers.js";

const providerCardsContainer = document.getElementById("provider-cards");
const providerFields = document.getElementById("provider-fields");
const embedProviderSlot = document.getElementById("embed-provider-picker");
const embedModelSlot = document.getElementById("embed-model-picker");
const themeSlot = document.getElementById("theme-picker");
const sendKeySlot = document.getElementById("send-key-picker");
const samplingNote = document.getElementById("sampling-note");
const savedFlag = document.getElementById("saved-flag");
const storageSummary = document.getElementById("storage-summary");
const storagePath = document.getElementById("storage-path");

const sliders = {
  temperature: ["temperature", "temperature-value", (value) => value.toFixed(1)],
  topP: ["top-p", "top-p-value", (value) => value.toFixed(2)],
  ragMinScore: ["rag-min-score", "min-score-value", (value) => value.toFixed(2)],
};

const numbers = {
  maxTokens: "max-tokens",
  historyLimit: "history-limit",
  ragTopK: "rag-top-k",
  chunkSize: "chunk-size",
  chunkOverlap: "chunk-overlap",
};

const state = { settings: null, pending: {}, timer: 0 };
const pickers = {};

export function currentSettings() {
  return state.settings;
}

export async function initSettings() {
  buildPickers();
  bindInputs();

  document.getElementById("open-folder").addEventListener("click", openFolder);
  document.getElementById("clear-chats").addEventListener("click", clearChats);
  document.addEventListener("model-picked", (event) => onModelPicked(event.detail));
  document.addEventListener("view-changed", (event) => {
    if (event.detail === "settings") refreshStorage();
  });

  try {
    apply(await api.getSettings());
  } catch (error) {
    toast(error, "error");
  }
}

function buildPickers() {
  pickers.embedProvider = createDropdown({
    onChange: (value) => {
      state.settings.embedProvider = value;
      queue({ embedProvider: value }, () => {
        pickers.embedModel.setItems(embedModelItems());
        pickers.embedModel.setValue(state.settings.embedModel);
      });
    },
  });
  embedProviderSlot.append(pickers.embedProvider.element);

  pickers.embedModel = createDropdown({
    searchable: true,
    allowCustom: true,
    onChange: (value) => {
      state.settings.embedModel = value;
      queue({ embedModel: value });
    },
  });
  embedModelSlot.append(pickers.embedModel.element);

  pickers.theme = createDropdown({
    items: [
      { value: "system", label: "Seguir o sistema" },
      { value: "light", label: "Claro" },
      { value: "dark", label: "Escuro" },
    ],
    onChange: (value) => {
      state.settings.theme = value;
      queue({ theme: value });
    },
  });
  themeSlot.append(pickers.theme.element);

  pickers.sendKey = createDropdown({
    items: [
      { value: "enter", label: "Enter", hint: "Shift+Enter quebra linha" },
      { value: "ctrl", label: "Ctrl+Enter", hint: "Enter quebra linha" },
    ],
    onChange: (value) => {
      state.settings.sendOnEnter = value === "enter";
      queue({ sendOnEnter: value === "enter" });
    },
  });
  sendKeySlot.append(pickers.sendKey.element);
}

function bindInputs() {
  const prompt = document.getElementById("system-prompt");
  prompt.addEventListener("change", () => queue({ systemPrompt: prompt.value }));

  for (const [key, [inputId, valueId, format]] of Object.entries(sliders)) {
    const input = document.getElementById(inputId);
    const readout = document.getElementById(valueId);

    input.addEventListener("input", () => {
      readout.textContent = format(Number(input.value));
    });
    input.addEventListener("change", () => queue({ [key]: Number(input.value) }));
  }

  for (const [key, inputId] of Object.entries(numbers)) {
    const input = document.getElementById(inputId);
    input.addEventListener("change", () => queue({ [key]: Number(input.value) }));
  }
}

function apply(settings) {
  state.settings = settings;

  renderProviderCards();

  pickers.embedProvider.setItems(
    settings.embedProviders.map((provider) => ({
      value: provider.id,
      label: provider.label,
      logoSrc: getProviderLogo(provider.id),
    }))
  );
  pickers.embedProvider.setValue(settings.embedProvider);
  pickers.embedModel.setItems(embedModelItems());
  pickers.embedModel.setValue(settings.embedModel);

  pickers.theme.setValue(settings.theme);
  pickers.sendKey.setValue(settings.sendOnEnter ? "enter" : "ctrl");

  document.getElementById("system-prompt").value = settings.systemPrompt;

  for (const [key, [inputId, valueId, format]] of Object.entries(sliders)) {
    document.getElementById(inputId).value = settings[key];
    document.getElementById(valueId).textContent = format(Number(settings[key]));
  }
  for (const [key, inputId] of Object.entries(numbers)) {
    document.getElementById(inputId).value = settings[key];
  }

  renderProviderFields();
  renderSamplingNote();
  broadcast();
}

function renderProviderCards() {
  if (!state.settings || !providerCardsContainer) return;
  const activeId = state.settings.provider;
  providerCardsContainer.replaceChildren();

  for (const provider of state.settings.providers) {
    const isActive = provider.id === activeId;
    const card = element("div", `provider-card ${isActive ? "active" : ""}`);

    // Top: Logo + Status Pill
    const top = element("div", "provider-card-top");
    const logoBox = element("div", "provider-card-logo");
    const logoSrc = getProviderLogo(provider.id);
    if (logoSrc) {
      const img = document.createElement("img");
      img.src = logoSrc;
      img.alt = provider.label;
      logoBox.append(img);
    }

    const badge = element("span", "provider-badge");
    const dot = element("span", "dot");
    let statusText = "Falta chave";

    if (!provider.needsKey) {
      dot.className = "dot local";
      statusText = "Local";
    } else if (provider.hasKey) {
      dot.className = "dot ok";
      statusText = "Pronto";
    } else {
      dot.className = "dot warn";
      statusText = "Sem chave";
    }
    badge.append(dot, element("span", null, statusText));
    top.append(logoBox, badge);

    // Bottom: Provider Name & Model
    const name = element("div", "provider-card-name", provider.label);
    const model = element("div", "provider-card-model", provider.model);

    card.append(top, name, model);

    card.addEventListener("click", () => {
      if (state.settings.provider === provider.id) return;
      state.settings.provider = provider.id;
      renderProviderCards();
      renderProviderFields();
      renderSamplingNote();
      queue({ provider: provider.id });
    });

    providerCardsContainer.append(card);
  }
}

function embedModelItems() {
  const provider = state.settings.embedProviders.find(
    (item) => item.id === state.settings.embedProvider
  );
  return (provider?.models ?? []).map((model) => ({ value: model, label: model }));
}

function activeProvider() {
  return state.settings.providers.find((item) => item.id === state.settings.provider);
}

function renderSamplingNote() {
  const provider = activeProvider();
  const ignores = provider && !provider.supportsSampling;

  samplingNote.hidden = !ignores;
  if (ignores) {
    samplingNote.querySelector("span:last-child").textContent =
      `${provider.label} ignora temperatura e top-p nos modelos atuais. Os dois valores valem para os outros provedores.`;
  }
}

function renderProviderFields() {
  const provider = activeProvider();
  if (!provider) return;

  providerFields.replaceChildren();

  // Header Banner with official logo and quick link
  const banner = element("div", "provider-details-header");
  const brand = element("div", "provider-details-brand");
  const logoSrc = getProviderLogo(provider.id);
  if (logoSrc) {
    const img = document.createElement("img");
    img.src = logoSrc;
    img.alt = "";
    brand.append(img);
  }
  brand.append(element("span", null, `Configuração de ${provider.label}`));
  banner.append(brand);

  if (provider.needsKey && provider.keyUrl) {
    const link = element("a", "provider-details-link", "Onde obter a chave");
    link.href = provider.keyUrl;
    link.setAttribute("data-external", "");
    link.append(icon("open_in_new"));
    banner.append(link);
  }

  providerFields.append(banner);

  if (provider.needsKey) providerFields.append(keyField(provider));
  providerFields.append(modelField(provider), baseUrlField(provider), testRow(provider));
}

function keyField(provider) {
  const field = element("label", "field");
  field.append(element("span", "field-label", "API Key"));

  const row = element("div", "key-field");
  const input = document.createElement("input");
  input.type = "password";
  input.spellcheck = false;
  input.placeholder = provider.hasKey ? `${provider.keyMasked} (salva)` : "Cole sua chave aqui";

  const reveal = element("button", "field-action-btn");
  reveal.type = "button";
  reveal.title = "Mostrar ou ocultar chave";
  reveal.append(icon("visibility"));
  reveal.addEventListener("click", () => {
    const hidden = input.type === "password";
    input.type = hidden ? "text" : "password";
    reveal.replaceChildren(icon(hidden ? "visibility_off" : "visibility"));
  });

  input.addEventListener("change", () => {
    const value = input.value.trim();
    if (value === "") return;
    input.value = "";
    queue({ apiKeys: { [provider.id]: value } }, () => {
      renderProviderCards();
      renderProviderFields();
      toast(`Chave de ${provider.label} salva.`);
    });
  });

  row.append(input, reveal);

  if (provider.hasKey) {
    const remove = element("button", "field-action-btn danger");
    remove.type = "button";
    remove.title = "Remover chave salva";
    remove.append(icon("delete"));
    remove.addEventListener("click", () => {
      queue({ apiKeys: { [provider.id]: "" } }, () => {
        renderProviderCards();
        renderProviderFields();
      });
    });
    row.append(remove);
  }

  field.append(row);

  const hint = element("span", "field-hint", "A chave completa nunca sai deste computador e não é enviada para o front-end.");
  field.append(hint);

  return field;
}

function modelField(provider) {
  const field = element("div", "field");
  field.append(element("span", "field-label", "Modelo"));

  const row = element("div", "split-field");
  const note = element("p", "inline-note");

  const dropdown = createDropdown({
    searchable: true,
    allowCustom: true,
    value: provider.model,
    items: modelItems(provider),
    onChange: (value) => {
      provider.model = value;
      renderProviderCards();
      queue({ models: { [provider.id]: value } });
    },
  });

  const refresh = element("button", "field-action-btn");
  refresh.type = "button";
  refresh.title = "Buscar os modelos que a sua chave enxerga";
  refresh.append(icon("refresh"));
  refresh.addEventListener("click", async () => {
    refresh.disabled = true;
    note.className = "inline-note";
    note.replaceChildren(icon("refresh"), element("span", null, "buscando modelos..."));

    try {
      const result = await api.listModels(provider.id);
      provider.models = result.models;
      dropdown.setItems(modelItems(provider));
      note.className = `inline-note ${result.live ? "ok" : "bad"}`;
      note.replaceChildren(
        icon(result.live ? "check_circle" : "error"),
        element("span", null, result.message)
      );
    } catch (error) {
      note.className = "inline-note bad";
      note.replaceChildren(icon("error"), element("span", null, String(error)));
    } finally {
      refresh.disabled = false;
    }
  });

  row.append(dropdown.element, refresh);
  field.append(row, note);
  return field;
}

function modelItems(provider) {
  const names = provider.models.includes(provider.model)
    ? provider.models
    : [provider.model, ...provider.models];
  return names.map((model) => ({ value: model, label: model }));
}

function baseUrlField(provider) {
  const field = element("label", "field");
  field.append(element("span", "field-label", "URL base"));

  const input = document.createElement("input");
  input.type = "text";
  input.spellcheck = false;
  input.value = provider.baseUrl;
  input.addEventListener("change", () => {
    const value = input.value.trim() || provider.defaultBaseUrl;
    input.value = value;
    provider.baseUrl = value;
    queue({ baseUrls: { [provider.id]: value } });
  });

  field.append(input);
  field.append(
    element("span", "field-hint", `Padrão: ${provider.defaultBaseUrl}`)
  );
  return field;
}

function testRow(provider) {
  const row = element("div", "button-row");
  const button = element("button", "button", "Testar conexão");
  button.type = "button";

  const result = element("span", "inline-note");

  button.addEventListener("click", async () => {
    button.disabled = true;
    result.className = "inline-note";
    result.replaceChildren(element("span", null, "testando conexão..."));

    try {
      const message = await api.checkProvider(provider.id);
      result.className = "inline-note ok";
      result.replaceChildren(icon("check_circle"), element("span", null, message));
    } catch (error) {
      result.className = "inline-note bad";
      result.replaceChildren(icon("error"), element("span", null, String(error)));
    } finally {
      button.disabled = false;
    }
  });

  row.append(button, result);
  return row;
}

function onModelPicked({ provider, model }) {
  state.settings.provider = provider;
  const target = state.settings.providers.find((item) => item.id === provider);
  if (target) target.model = model;

  renderProviderCards();
  renderProviderFields();
  renderSamplingNote();
  queue({ provider, models: { [provider]: model } });
}

async function refreshStorage() {
  try {
    const info = await api.storageInfo();
    storagePath.textContent = info.folder;
    storageSummary.replaceChildren(
      stat("Tamanho do banco", formatBytes(info.sizeBytes)),
      stat("Conversas", String(info.chats)),
      stat("Mensagens", String(info.messages)),
      stat("Documentos", String(info.documents)),
      stat("Trechos", String(info.chunks))
    );
  } catch (error) {
    toast(error, "error");
  }
}

async function openFolder() {
  try {
    await api.openDataFolder();
  } catch (error) {
    toast(error, "error");
  }
}

async function clearChats() {
  const confirmed = await confirm(
    "Apagar todas as conversas? Os documentos indexados continuam.",
    { title: "HoChat", kind: "warning", okLabel: "Apagar", cancelLabel: "Cancelar" }
  );
  if (!confirmed) return;

  try {
    await api.clearAllChats();
    document.dispatchEvent(new CustomEvent("chats-cleared"));
    await refreshStorage();
    toast("Conversas apagadas.");
  } catch (error) {
    toast(error, "error");
  }
}

function broadcast() {
  document.dispatchEvent(
    new CustomEvent("settings-changed", { detail: state.settings })
  );
}

function queue(patch, after) {
  Object.assign(state.pending, patch, {
    apiKeys: { ...state.pending.apiKeys, ...patch.apiKeys },
    baseUrls: { ...state.pending.baseUrls, ...patch.baseUrls },
    models: { ...state.pending.models, ...patch.models },
  });

  clearTimeout(state.timer);
  state.timer = setTimeout(async () => {
    const payload = state.pending;
    state.pending = {};

    try {
      state.settings = await api.saveSettings(payload);
      broadcast();
      flash();
      if (after) after();
    } catch (error) {
      toast(error, "error");
    }
  }, 300);
}

function flash() {
  savedFlag.classList.add("visible");
  clearTimeout(flash.timer);
  flash.timer = setTimeout(() => savedFlag.classList.remove("visible"), 1400);
}
