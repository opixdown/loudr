import React, { useState } from "react";
import ReactDOM from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useSetupState } from "./lib/hooks";
import { api, on, type Settings as Settings_ } from "./lib/ipc";
import Onboarding from "./windows/Onboarding";
import Pill from "./windows/Pill";
import Settings from "./windows/Settings";
import "./styles.css";

// One bundle serves both windows; the window label picks what to render.
const label = getCurrentWindow().label;
document.documentElement.dataset.window = label;

// Follow the Appearance setting in both windows. Without a data-theme
// attribute the CSS follows the system's light / dark preference.
function applyTheme(settings: Settings_) {
  if (settings.theme === "system") delete document.documentElement.dataset.theme;
  else document.documentElement.dataset.theme = settings.theme;
}
api.getSettings().then(applyTheme);
on<Settings_>("settings-changed", applyTheme);

function SettingsWindow() {
  const setup = useSetupState();
  const [justFinished, setJustFinished] = useState(false);
  if (!setup) return null;
  return setup.onboarded || justFinished ? (
    <Settings setup={setup} />
  ) : (
    <Onboarding setup={setup} onDone={() => setJustFinished(true)} />
  );
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>{label === "pill" ? <Pill /> : <SettingsWindow />}</React.StrictMode>,
);
