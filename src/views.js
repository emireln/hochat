const views = new Map(
  [...document.querySelectorAll(".view")].map((view) => [view.dataset.view, view])
);
const groups = [...document.querySelectorAll(".topbar-group")];
const navItems = [...document.querySelectorAll(".nav-item")];
const titleNode = document.getElementById("topbar-title");
const backButton = document.getElementById("back-to-chat");

const titles = {
  docs: "Base de conhecimento",
  settings: "Configurações",
};

let current = "chat";
let chatTitle = "Nova conversa";

export function currentView() {
  return current;
}

export function setChatTitle(text) {
  chatTitle = text;
  if (current === "chat") titleNode.textContent = text;
}

export function showView(name) {
  if (!views.has(name) || name === current) return;

  views.get(current).hidden = true;
  const view = views.get(name);
  view.hidden = false;
  view.classList.remove("entering");
  void view.offsetWidth;
  view.classList.add("entering");
  current = name;

  titleNode.textContent = name === "chat" ? chatTitle : titles[name];
  backButton.hidden = name === "chat";
  for (const group of groups) group.hidden = group.dataset.for !== name;
  for (const item of navItems) item.classList.toggle("active", item.dataset.goto === name);

  document.dispatchEvent(new CustomEvent("view-changed", { detail: name }));
}
