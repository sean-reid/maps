import type { SeriesMeta } from "./data.ts";
import { prepare, search } from "./search.ts";

/**
 * Wires a text input and a list into a combobox over the series index.
 * `onPick` hears the chosen series once the box has emptied and closed; it decides where focus goes.
 */
export function createSearchBox(
  input: HTMLInputElement,
  list: HTMLElement,
  series: SeriesMeta[],
  onPick: (meta: SeriesMeta) => void,
): void {
  const index = prepare(series);
  let results: SeriesMeta[] = [];
  let active = -1;

  input.placeholder = `Search ${series.length.toLocaleString("en")} series`;
  input.disabled = false;

  const close = (): void => {
    list.hidden = true;
    active = -1;
    input.setAttribute("aria-expanded", "false");
    input.removeAttribute("aria-activedescendant");
  };

  const mark = (next: number): void => {
    active = next;
    [...list.children].forEach((item, i) => {
      item.setAttribute("aria-selected", String(i === active));
      if (i === active) {
        input.setAttribute("aria-activedescendant", item.id);
        item.scrollIntoView({ block: "nearest" });
      }
    });
    if (active < 0) input.removeAttribute("aria-activedescendant");
  };

  const render = (): void => {
    results = search(index, input.value);
    list.replaceChildren(
      ...results.map((meta, i) => {
        const item = document.createElement("li");
        item.id = `result-${i}`;
        item.setAttribute("role", "option");
        item.setAttribute("aria-selected", "false");
        const name = document.createElement("span");
        name.textContent = meta.title;
        const facts = document.createElement("span");
        facts.className = "soft";
        const span =
          meta.first === meta.last ? String(meta.first) : `${meta.first} to ${meta.last}`;
        facts.textContent = [meta.unit, span, `${meta.entities} countries`]
          .filter(Boolean)
          .join(", ");
        item.append(name, " ", facts);
        return item;
      }),
    );
    if (results.length === 0) {
      const none = document.createElement("li");
      none.setAttribute("role", "option");
      none.setAttribute("aria-disabled", "true");
      none.setAttribute("aria-selected", "false");
      none.className = "soft";
      none.textContent = "No series by that name";
      list.append(none);
    }
    list.hidden = false;
    active = -1;
    input.setAttribute("aria-expanded", "true");
    input.removeAttribute("aria-activedescendant");
  };

  const pick = (meta: SeriesMeta | undefined): void => {
    if (!meta) return;
    input.value = "";
    close();
    onPick(meta);
  };

  input.addEventListener("focus", render);
  input.addEventListener("input", render);
  input.addEventListener("keydown", (event) => {
    // Enter and the arrows belong to the input method while it is composing.
    if (event.isComposing) return;
    if (event.key === "Escape") {
      if (list.hidden) input.value = "";
      close();
    } else if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      if (list.hidden) render();
      if (results.length === 0) return;
      const step = event.key === "ArrowDown" ? 1 : -1;
      mark((active + step + results.length) % results.length);
    } else if (event.key === "Enter") {
      event.preventDefault();
      pick(results[active < 0 ? 0 : active]);
    }
  });
  // Pressing an option must not blur the input first, or the list closes before the click lands.
  list.addEventListener("pointerdown", (event) => event.preventDefault());
  list.addEventListener("click", (event) => {
    const item = (event.target as Element).closest("li[role=option]:not([aria-disabled])");
    if (item) pick(results[[...list.children].indexOf(item)]);
  });
  input.addEventListener("blur", close);
}
