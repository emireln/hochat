import { getCurrentWindow } from "@tauri-apps/api/window";
import { openUrl } from "@tauri-apps/plugin-opener";
import { initChat } from "./chat.js";
import { initDocs, refreshDocs } from "./docs.js";
import { currentSettings, initSettings } from "./settings.js";
import { applyTheme, autosize, icon, watchSystemTheme } from "./ui.js";
import { currentView, showView } from "./views.js";

const app = document.getElementById("app");
const toggle = document.getElementById("toggle-sidebar");
const prompt = document.getElementById("prompt");

// Controles da janela customizada (Tauri)
const win = getCurrentWindow();
const winMinimize = document.getElementById("win-minimize");
const winMaximize = document.getElementById("win-maximize");
const winClose = document.getElementById("win-close");
const iconMaximize = winMaximize?.querySelector(".icon-maximize");
const iconRestore = winMaximize?.querySelector(".icon-restore");

async function updateMaximizedState() {
  try {
    const isMax = await win.isMaximized();
    if (iconMaximize && iconRestore) {
      iconMaximize.style.display = isMax ? "none" : "block";
      iconRestore.style.display = isMax ? "block" : "none";
      winMaximize.title = isMax ? "Restaurar" : "Maximizar";
    }
  } catch {
    // Ignora caso não esteja rodando dentro do Tauri runtime
  }
}

winMinimize?.addEventListener("click", () => win.minimize());
winMaximize?.addEventListener("click", async () => {
  await win.toggleMaximize();
  await updateMaximizedState();
});
winClose?.addEventListener("click", () => win.close());

document.getElementById("window-titlebar")?.addEventListener("dblclick", async (event) => {
  if (event.target.closest(".window-controls")) return;
  await win.toggleMaximize();
  await updateMaximizedState();
});

try {
  win.onResized(() => updateMaximizedState());
  updateMaximizedState();
} catch {
  // Ambiente de desenvolvimento web
}

function paintSidebar(collapsed) {
  app.classList.toggle("collapsed", collapsed);
  toggle.replaceChildren(icon(collapsed ? "left_panel_open" : "left_panel_close"));
  toggle.title = collapsed
    ? "Mostrar barra lateral (Ctrl+B)"
    : "Recolher barra lateral (Ctrl+B)";
}

function toggleSidebar() {
  const collapsed = !app.classList.contains("collapsed");
  localStorage.setItem("hochat.sidebar", collapsed ? "collapsed" : "open");
  paintSidebar(collapsed);
}

paintSidebar(localStorage.getItem("hochat.sidebar") === "collapsed");
toggle.addEventListener("click", toggleSidebar);

document.addEventListener("click", (event) => {
  const link = event.target.closest("a[data-external]");
  if (link) {
    event.preventDefault();
    openUrl(link.href);
    return;
  }

  const target = event.target.closest("[data-goto]");
  if (target) showView(target.dataset.goto);
});

document.addEventListener("view-changed", (event) => {
  if (event.detail === "docs") refreshDocs();
  if (event.detail === "chat") prompt.focus();
});

document.addEventListener("settings-changed", (event) => {
  applyTheme(event.detail.theme);
});

document.addEventListener("keydown", (event) => {
  if (event.key === "Escape" && currentView() !== "chat") {
    showView("chat");
    return;
  }
  if (!event.ctrlKey) return;

  if (event.key === "b") {
    event.preventDefault();
    toggleSidebar();
  }
  if (event.key === "n") {
    event.preventDefault();
    showView("chat");
    document.getElementById("new-chat").click();
  }
  if (event.key === ",") {
    event.preventDefault();
    showView("settings");
  }
});

watchSystemTheme(() => currentSettings()?.theme ?? "system");

await initSettings();
await initChat();
await initDocs();

autosize(prompt);
prompt.focus();
