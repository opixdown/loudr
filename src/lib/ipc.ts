// Typed wrappers around the Rust commands and events.
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type Settings = {
  hotkey_code: number;
  speak_key: number;
  voice: string;
  voice_speed: number;
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
  /** Name of the read-aloud key. */
  speak_label: string;
  /** The text-to-speech voice is downloaded. */
  voice_ready: boolean;
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

export type Voices = {
  voices: { id: string; label: string }[];
  download_mb: number;
};

/** What the speech worker is doing. */
export type SpeechEvent = {
  state: "idle" | "preparing" | "speaking" | "exporting";
  /** 0 to 1 while exporting. */
  progress: number;
  error?: string | null;
  /** Path of the audio file an export just wrote. */
  saved?: string | null;
  /** Start of the text read aloud last, whose audio can be saved. */
  last?: string | null;
};

export type Status =
  | { state: "idle" | "recording" | "transcribing" | "preparing" | "speaking" }
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
  startHotkeyCapture: (target: "hold" | "speak") =>
    invoke<void>("start_hotkey_capture", { target }),
  cancelHotkeyCapture: () => invoke<void>("cancel_hotkey_capture"),
  listVoices: () => invoke<Voices>("list_voices"),
  downloadVoice: () => invoke<void>("download_voice"),
  speakText: (text: string) => invoke<void>("speak_text", { text }),
  stopSpeaking: () => invoke<void>("stop_speaking"),
  exportSpeech: (text: string) => invoke<void>("export_speech", { text }),
  saveLastSpeech: () => invoke<void>("save_last_speech"),
  lastSpeech: () => invoke<string | null>("last_speech"),
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
