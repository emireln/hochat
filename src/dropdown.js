import { element, icon } from "./ui.js";

let openMenu = null;

export function createDropdown(options) {
  const config = {
    items: [],
    value: null,
    placeholder: "Selecione",
    searchable: false,
    allowCustom: false,
    compact: false,
    onChange: () => {},
    ...options,
  };

  const root = element("div", config.compact ? "dropdown compact" : "dropdown");
  const trigger = element("button", "dropdown-trigger");
  trigger.type = "button";
  trigger.setAttribute("aria-haspopup", "listbox");
  trigger.setAttribute("aria-expanded", "false");

  const logoNode = document.createElement("img");
  logoNode.className = "dropdown-trigger-logo";
  logoNode.style.display = "none";

  const label = element("span", "dropdown-value");
  trigger.append(logoNode, label, icon("expand_more"));
  root.append(trigger);

  let items = config.items;
  let value = config.value;

  function paintTrigger() {
    const current = items.find((item) => item.value === value);
    const text = current ? current.label : value;
    if (current && current.logoSrc) {
      logoNode.src = current.logoSrc;
      logoNode.style.display = "block";
    } else {
      logoNode.style.display = "none";
    }
    label.textContent = text || config.placeholder;
    label.classList.toggle("placeholder", !text);
    trigger.title = text || config.placeholder;
  }

  function choose(next) {
    if (next === value) return close();
    value = next;
    paintTrigger();
    close();
    config.onChange(next);
  }

  function close() {
    if (openMenu && openMenu.root === root) openMenu.dismiss();
  }

  trigger.addEventListener("click", () => {
    if (openMenu && openMenu.root === root) {
      close();
      return;
    }
    openDropdown({ root, trigger, items, value, config, choose });
  });

  paintTrigger();

  return {
    element: root,
    getValue: () => value,
    setValue(next) {
      value = next;
      paintTrigger();
    },
    setItems(next) {
      items = next;
      paintTrigger();
    },
  };
}

function openDropdown({ root, trigger, items, value, config, choose }) {
  if (openMenu) openMenu.dismiss();

  const menu = element("div", "dropdown-menu");
  menu.setAttribute("role", "listbox");

  const list = element("div", "dropdown-list");
  let search = null;

  if (config.searchable) {
    const box = element("div", "dropdown-search");
    search = document.createElement("input");
    search.type = "text";
    search.placeholder = config.allowCustom ? "Buscar ou digitar" : "Buscar";
    search.spellcheck = false;
    box.append(icon("search"), search);
    menu.append(box);
  }

  menu.append(list);
  document.body.append(menu);

  let highlighted = -1;
  let rows = [];

  function paintList() {
    const term = search ? search.value.trim().toLowerCase() : "";
    const visible = term
      ? items.filter((item) => item.label.toLowerCase().includes(term))
      : items;

    const entries = [...visible];
    if (config.allowCustom && term && !items.some((item) => item.value === search.value.trim())) {
      entries.unshift({ value: search.value.trim(), label: search.value.trim(), hint: "usar assim" });
    }

    rows = entries.map((item) => {
      const row = element("button", "dropdown-item");
      row.type = "button";
      row.setAttribute("role", "option");
      row.setAttribute("aria-selected", String(item.value === value));

      if (item.logoSrc) {
        const img = document.createElement("img");
        img.src = item.logoSrc;
        img.className = "dropdown-item-logo";
        row.append(img);
      }

      const body = element("div", "dropdown-item-body");
      body.append(element("span", "dropdown-item-label", item.label));
      if (item.hint) body.append(element("span", "dropdown-item-hint", item.hint));

      row.append(body, icon("check"));
      row.addEventListener("click", () => choose(item.value));
      return row;
    });

    if (rows.length === 0) {
      list.replaceChildren(element("p", "dropdown-empty", "Nada encontrado"));
      return;
    }

    list.replaceChildren(...rows);
    highlighted = Math.max(
      0,
      entries.findIndex((item) => item.value === value)
    );
    paintHighlight();
  }

  function paintHighlight() {
    rows.forEach((row, index) => row.classList.toggle("highlighted", index === highlighted));
    rows[highlighted]?.scrollIntoView({ block: "nearest" });
  }

  function move(step) {
    if (rows.length === 0) return;
    highlighted = (highlighted + step + rows.length) % rows.length;
    paintHighlight();
  }

  function onKeydown(event) {
    if (event.key === "Escape") {
      event.preventDefault();
      dismiss();
      trigger.focus();
    } else if (event.key === "ArrowDown") {
      event.preventDefault();
      move(1);
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      move(-1);
    } else if (event.key === "Enter") {
      event.preventDefault();
      if (rows[highlighted]) rows[highlighted].click();
    }
  }

  function onOutside(event) {
    if (root.contains(event.target) || menu.contains(event.target)) return;
    dismiss();
  }

  function onReposition() {
    dismiss();
  }

  function place() {
    const rect = trigger.getBoundingClientRect();
    const spaceBelow = window.innerHeight - rect.bottom;
    const spaceAbove = rect.top;
    const upward = spaceBelow < 240 && spaceAbove > spaceBelow;

    menu.classList.toggle("upward", upward);
    menu.style.minWidth = `${rect.width}px`;
    menu.style.left = `${Math.min(rect.left, window.innerWidth - 240)}px`;

    if (upward) {
      menu.style.bottom = `${window.innerHeight - rect.top + 4}px`;
      menu.style.top = "auto";
    } else {
      menu.style.top = `${rect.bottom + 4}px`;
      menu.style.bottom = "auto";
    }
  }

  function dismiss() {
    document.removeEventListener("pointerdown", onOutside);
    document.removeEventListener("keydown", onKeydown);
    window.removeEventListener("resize", onReposition);
    window.removeEventListener("scroll", onReposition, true);
    trigger.setAttribute("aria-expanded", "false");
    menu.remove();
    openMenu = null;
  }

  place();
  paintList();
  trigger.setAttribute("aria-expanded", "true");

  document.addEventListener("pointerdown", onOutside);
  document.addEventListener("keydown", onKeydown);
  window.addEventListener("resize", onReposition);
  window.addEventListener("scroll", onReposition, true);

  if (search) search.focus();

  openMenu = { root, dismiss };
}
