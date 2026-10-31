import React from "react";
import ReactDOM from "react-dom/client";
import "@fontsource/inter-tight/latin-300.css";
import "@fontsource/inter-tight/latin-600.css";
import "@fontsource/inter-tight/latin-800.css";
import "@fontsource/dm-sans/latin-300.css";
import "@fontsource/dm-sans/latin-600.css";
import "@fontsource/dm-sans/latin-800.css";
import "@fontsource/manrope/latin-300.css";
import "@fontsource/manrope/latin-600.css";
import "@fontsource/manrope/latin-800.css";
import "@fontsource/poppins/latin-300.css";
import "@fontsource/poppins/latin-600.css";
import "@fontsource/poppins/latin-800.css";
import "@fontsource/montserrat/latin-300.css";
import "@fontsource/montserrat/latin-600.css";
import "@fontsource/montserrat/latin-800.css";
import "@fontsource/rubik/latin-300.css";
import "@fontsource/rubik/latin-600.css";
import "@fontsource/rubik/latin-800.css";
import "@fontsource/lexend/latin-300.css";
import "@fontsource/lexend/latin-600.css";
import "@fontsource/lexend/latin-800.css";
import "@fontsource/outfit/latin-300.css";
import "@fontsource/outfit/latin-600.css";
import "@fontsource/outfit/latin-800.css";
import "@fontsource/sora/latin-300.css";
import "@fontsource/sora/latin-600.css";
import "@fontsource/sora/latin-800.css";
import "@fontsource/nunito/latin-300.css";
import "@fontsource/nunito/latin-700.css";
import "@fontsource/nunito/latin-900.css";
import "@fontsource/space-grotesk/latin-300.css";
import "@fontsource/space-grotesk/latin-600.css";
import "@fontsource/space-grotesk/latin-700.css";
import "@fontsource/oswald/latin-300.css";
import "@fontsource/oswald/latin-500.css";
import "@fontsource/oswald/latin-700.css";
import "@fontsource/barlow-condensed/latin-300.css";
import "@fontsource/barlow-condensed/latin-600.css";
import "@fontsource/barlow-condensed/latin-800.css";
import "@fontsource/bebas-neue/latin-400.css";
import "@fontsource/unbounded/latin-300.css";
import "@fontsource/unbounded/latin-600.css";
import "@fontsource/unbounded/latin-800.css";
import "@fontsource/orbitron/latin-400.css";
import "@fontsource/orbitron/latin-600.css";
import "@fontsource/orbitron/latin-800.css";
import "@fontsource/jetbrains-mono/latin-300.css";
import "@fontsource/jetbrains-mono/latin-600.css";
import "@fontsource/jetbrains-mono/latin-800.css";
import "@fontsource/ibm-plex-mono/latin-300.css";
import "@fontsource/ibm-plex-mono/latin-600.css";
import "@fontsource/ibm-plex-mono/latin-700.css";
import "@fontsource/space-mono/latin-400.css";
import "@fontsource/space-mono/latin-700.css";
import "@fontsource/inter-tight/latin-400.css";
import "@fontsource/jetbrains-mono/latin-400.css";
import "./skins/skins.css";
import { Board } from "./forms/Board";
import { EdgeTab } from "./forms/EdgeTab";
import { Island } from "./forms/Island";
import { Overlay } from "./forms/Overlay";
import { Pop } from "./forms/Pop";
import { Recap } from "./forms/Recap";
import { BOARD_IDS } from "./forms/registry";
import { Settings } from "./settings/Settings";

const which = new URLSearchParams(window.location.search).get("w") ?? "settings";
const widgets: Record<string, () => React.ReactElement> = {
  island: () => <Island />,
  edge: () => <EdgeTab />,
  pop: () => <Pop />,
  recap: () => <Recap />,
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
