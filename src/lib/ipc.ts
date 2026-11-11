// Typed wrappers around the Rust commands and events.
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type Settings = {
  hotkey_code: number;
  theme: "system" | "light" | "dark";
  model: string;
  language: string;
  input_device: string | null;
  launch_at_login: boolean;
  onboarded: boolean;
};

export type SetupState = {
  /** Name of the hold-to-talk key, e.g. "Right Option". */
  hotkey_label: string;
  platform: "macos" | "windows" | string;
  onboarded: boolean;
  microphone: "granted" | "denied" | "unknown";
  accessibility: boolean;
  model_ready: boolean;
  downloading: boolean;
};

export type ModelInfo = {
  id: string;
  label: string;
  size_mb: number;
  downloaded: boolean;
};

export type DownloadProgress = {
  file: string;
  downloaded: number;
  total: number;
  done: boolean;
  error: string | null;
};

export type Status =
  | { state: "idle" | "recording" | "transcribing" }
  | { state: "error"; message: string };

export const api = {
  getSettings: () => invoke<Settings>("get_settings"),
  setSettings: (settings: Settings) => invoke<void>("set_settings", { settings }),
  getSetupState: () => invoke<SetupState>("get_setup_state"),
  requestMicrophone: () => invoke<void>("request_microphone"),
  requestAccessibility: () => invoke<void>("request_accessibility"),
  openPrivacySettings: (pane: "microphone" | "accessibility") =>
    invoke<void>("open_privacy_settings", { pane }),
  listInputDevices: () => invoke<string[]>("list_input_devices"),
  listModels: () => invoke<ModelInfo[]>("list_models"),
  downloadModel: () => invoke<void>("download_model"),
  finishOnboarding: () => invoke<void>("finish_onboarding"),
  startHotkeyCapture: () => invoke<void>("start_hotkey_capture"),
  cancelHotkeyCapture: () => invoke<void>("cancel_hotkey_capture"),
};

/** Subscribe to a backend event; returns a cleanup function for useEffect. */
export function on<T>(event: string, handler: (payload: T) => void): () => void {
  let unlisten: UnlistenFn | undefined;
  let cancelled = false;
  listen<T>(event, (e) => handler(e.payload)).then((fn) => {
    if (cancelled) fn();
    else unlisten = fn;
  });
  return () => {
    cancelled = true;
    unlisten?.();
  };
}
