import React from "react";
import ReactDOM from "react-dom/client";
import "@fontsource/inter-tight/300.css";
import "@fontsource/inter-tight/400.css";
import "@fontsource/inter-tight/600.css";
import "@fontsource/inter-tight/800.css";
import "@fontsource/oswald/300.css";
import "@fontsource/oswald/600.css";
import "@fontsource/jetbrains-mono/400.css";
import "@fontsource/jetbrains-mono/600.css";
import "@fontsource/nunito/800.css";
import "@fontsource/fraunces/600.css";
import "@fontsource/outfit/600.css";
import "@fontsource/sora/600.css";
import "@fontsource/bebas-neue/400.css";
import "@fontsource/unbounded/600.css";
import "@fontsource/space-grotesk/700.css";
import "@fontsource/orbitron/700.css";
import "@fontsource/playfair-display/700-italic.css";
import "@fontsource/dm-serif-display/400.css";
import "./skins/skins.css";
import { Board } from "./forms/Board";
import { EdgeTab } from "./forms/EdgeTab";
import { Island } from "./forms/Island";
import { Overlay } from "./forms/Overlay";
import { Pop } from "./forms/Pop";
import { BOARD_IDS } from "./forms/registry";
import { Settings } from "./settings/Settings";

const which = new URLSearchParams(window.location.search).get("w") ?? "settings";
const widgets: Record<string, () => React.ReactElement> = {
  island: () => <Island />,
  edge: () => <EdgeTab />,
  pop: () => <Pop />,
  overlay: () => <Overlay />,
};
for (const id of BOARD_IDS) widgets[id] = () => <Board id={id} />;

async function boot() {
  if (widgets[which]) {
    document.documentElement.classList.add("widget");
    await import("./forms/forms.css");
    await import("./forms/boards.css");
  } else {
    await import("./forms/forms.css");
    await import("./forms/boards.css");
    await import("./settings/settings.css");
  }
  document.addEventListener("contextmenu", (e) => {
    if (!(e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement)) e.preventDefault();
  });
  ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>{widgets[which] ? widgets[which]() : <Settings />}</React.StrictMode>,
  );
}

void boot();
