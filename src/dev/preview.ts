// Browser preview of the UI without the Rust backend, for working on the
// design: run `npm run dev` and open http://localhost:1420/preview.html
//
// Query parameters pick the state to show:
//   window=settings|pill   tab=dictation|speech   voice=0|1   onboarded=0|1
//   theme=light|dark       status=recording|transcribing|preparing|speaking
//   last=<text read aloud last>
import { emit } from "@tauri-apps/api/event";
import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";

const q = new URLSearchParams(location.search);
const flag = (name: string, fallback: boolean) =>
  q.has(name) ? q.get(name) === "1" : fallback;

const settings = {
  hotkey_code: 61,
  speak_key: 54,
  voice: "am_puck",
  voice_speed: 1,
  theme: q.get("theme") ?? "system",
  model: "base",
  language: "en",
  input_device: null,
  launch_at_login: false,
  onboarded: flag("onboarded", true),
};

mockWindows(q.get("window") ?? "settings");
mockIPC(
  (cmd, args) => {
    switch (cmd) {
      case "get_settings":
        return settings;
      case "set_settings":
        Object.assign(settings, (args as { settings: typeof settings }).settings);
        return;
      case "get_setup_state":
        return {
          hotkey_label: "Right Option",
          speak_label: "Right Command",
          voice_ready: flag("voice", true),
          platform: "macos",
          onboarded: settings.onboarded,
          microphone: "granted",
          accessibility: true,
          model_ready: true,
          downloading: false,
        };
      case "list_input_devices":
        return ["MacBook Pro Microphone"];
      case "list_models":
        return [
          { id: "tiny", label: "Tiny", size_mb: 75, downloaded: false },
          { id: "base", label: "Base", size_mb: 142, downloaded: true },
          { id: "small", label: "Small", size_mb: 466, downloaded: false },
          { id: "medium", label: "Medium", size_mb: 1500, downloaded: false },
        ];
      case "list_voices":
        return {
          voices: [
            { id: "am_puck", label: "Puck · American, male" },
            { id: "af_heart", label: "Heart · American, female" },
            { id: "bm_george", label: "George · British, male" },
          ],
          download_mb: 205,
        };
      case "last_speech":
        return q.get("last");
    }
  },
  { shouldMockEvents: true },
);

await import("../main");
// Give the components a moment to subscribe, then put them in the asked state.
setTimeout(() => {
  if (q.has("tab")) emit("navigate", q.get("tab"));
  if (q.has("status")) emit("status", { state: q.get("status") });
}, 300);
