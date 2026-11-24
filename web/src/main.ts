import "@fontsource/inter/latin-400.css";
import "@fontsource/inter/latin-600.css";
import "@fontsource/jetbrains-mono/latin-400.css";
import "./styles/tokens.css";
import "./styles/app.css";

import * as core from "./core";
import { download, h } from "./dom";
import { cards } from "./components/cards";
import { chartView } from "./components/chart";
import { gridView } from "./components/grids";
import { gridControls, modeToggle } from "./components/mode";
import { summaryView } from "./components/summary";
import { header, thesisBar } from "./components/thesis";
import { store } from "./state";

core.init();
store.loadFromHash(location.hash);

function actions(): HTMLElement {
  const copy = h("button", { type: "button", class: "ghost" }, "Copy link");
  copy.addEventListener("click", () => {
    const link = store.link();
    void navigator.clipboard?.writeText(link).then(
      () => (copy.textContent = "Link copied"),
      () => window.prompt("Copy this link", link),
    );
    setTimeout(() => (copy.textContent = "Copy link"), 2000);
  });
  const csv = h("button", { type: "button", class: "ghost" }, "Download CSV");
  csv.addEventListener("click", () => {
    try {
      download("thesisgrid.csv", new Blob([core.csv(store.sc)], { type: "text/csv" }));
    } catch {
      /* the error banner already explains what to fix */
    }
  });
  return h("div", { class: "actions" }, copy, csv);
}

function footer(): HTMLElement {
  const asOf = core.presets().as_of;
  return h(
    "footer",
    {},
    h("p", {}, "Not financial advice. Numbers are model estimates from your inputs."),
    h("p", { class: "muted" }, `Venue presets as of ${asOf}. Every preset value can be overridden.`),
  );
}

const app = document.getElementById("app");
if (app) {
  app.replaceChildren(
    header(),
    h(
      "main",
      {},
      thesisBar(),
      modeToggle(),
      cards(),
      h("div", { class: "section-head" }, h("h2", {}, "Grids"), actions()),
      gridControls(),
      gridView(),
      summaryView(),
      chartView(),
    ),
    footer(),
  );
  store.run(true);
}
