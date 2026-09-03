import { getCurrentWindow } from "@tauri-apps/api/window";

const toasts = document.getElementById("toasts");
const darkQuery = window.matchMedia("(prefers-color-scheme: dark)");

export function element(tag, className, text) {
  const node = document.createElement(tag);
  if (className) node.className = className;
  if (text !== undefined) node.textContent = text;
  return node;
}

export function icon(name) {
  return element("span", "icon", name);
}

export function toast(message, kind = "info") {
  const node = element("div", `toast ${kind}`);
  node.append(icon(kind === "error" ? "error" : "check_circle"));
  node.append(element("span", null, String(message)));
  toasts.append(node);

  setTimeout(
    () => {
      node.classList.add("leaving");
      node.addEventListener("animationend", () => node.remove(), { once: true });
    },
    kind === "error" ? 6000 : 2600
  );
}

export function applyTheme(theme) {
  const dark = theme === "dark" || (theme === "system" && darkQuery.matches);
  document.documentElement.dataset.theme = dark ? "dark" : "light";
  getCurrentWindow()
    .setTheme(dark ? "dark" : "light")
    .catch(() => {});
}

export function watchSystemTheme(getTheme) {
  darkQuery.addEventListener("change", () => {
    if (getTheme() === "system") applyTheme("system");
  });
}

export function stat(label, value) {
  const box = element("div", "stat");
  box.append(element("span", null, label), element("b", null, value));
  return box;
}

export function formatBytes(bytes) {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)} KB`;
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}

export function formatDate(milliseconds) {
  return new Date(milliseconds).toLocaleDateString("pt-BR", {
    day: "2-digit",
    month: "short",
    year: "numeric",
  });
}

export function autosize(textarea) {
  textarea.style.height = "auto";
  textarea.style.height = `${Math.min(textarea.scrollHeight, 200)}px`;
}
