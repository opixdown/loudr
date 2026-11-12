// Text to speech tab: the read-aloud key, the voice, and a box for turning
// a script into speech or an audio file.
import { useEffect, useState } from "react";
import {
  api,
  on,
  type DownloadProgress,
  type Settings,
  type SetupState,
  type SpeechEvent,
  type Voices,
} from "../lib/ipc";
import { KeyRow } from "./Settings";
import { DownloadBar } from "./Setup";

const IDLE: SpeechEvent = { state: "idle", progress: 0 };
// Kept outside the component so the script survives switching tabs.
let draft = "";

export default function Speech({
  setup,
  settings,
  update,
}: {
  setup: SetupState;
  settings: Settings;
  update: (patch: Partial<Settings>) => void;
}) {
  const [voices, setVoices] = useState<Voices | null>(null);
  const [download, setDownload] = useState<DownloadProgress | null>(null);
  const [speech, setSpeech] = useState<SpeechEvent>(IDLE);
  // The last error or saved file stays on screen after the worker goes idle.
  const [result, setResult] = useState<{ error?: string; saved?: string }>({});
  const [text, setText] = useState(draft);

  useEffect(() => {
    api.listVoices().then(setVoices);
    const offDownload = on<DownloadProgress>("voice-download", (p) =>
      setDownload(p.done ? null : p),
    );
    const offSpeech = on<SpeechEvent>("speech", (event) => {
      setSpeech(event);
      if (event.error) setResult({ error: event.error });
      else if (event.saved) setResult({ saved: event.saved });
    });
    return () => {
      offDownload();
      offSpeech();
    };
  }, []);
  if (!voices) return null;

  if (!setup.voice_ready) {
    const busy = setup.downloading || (download !== null && !download.error);
    return (
      <section>
        <div className="row">
          <div className="row-text">
            <strong>Voice</strong>
            <span>
              Loudr can read text aloud and turn a script into an audio file, with a natural
              voice that runs on this computer. One-time download, about {voices.download_mb} MB.
            </span>
          </div>
          <button
            className="primary"
            disabled={busy}
            onClick={() => {
              setDownload(null);
              api.downloadVoice().catch(() => {});
            }}
          >
            {busy ? "Downloading…" : "Download"}
          </button>
        </div>
        <DownloadBar progress={download} />
      </section>
    );
  }

  const working = speech.state !== "idle";
  const empty = text.trim() === "";
  const start = (action: (text: string) => Promise<void>) => {
    setResult({});
    action(text);
  };

  return (
    <>
      <section>
        <KeyRow
          target="speak"
          title="Read aloud"
          hint="Select text in any app, then tap this key. Tap again to stop."
          label={setup.speak_label}
        />

        <label>
          <span>Voice</span>
          <select value={settings.voice} onChange={(e) => update({ voice: e.target.value })}>
            {voices.voices.map((v) => (
              <option key={v.id} value={v.id}>
                {v.label}
              </option>
            ))}
          </select>
        </label>

        <label>
          <span>Speed</span>
          <div className="speed">
            <input
              type="range"
              min={0.7}
              max={1.5}
              step={0.05}
              value={settings.voice_speed}
              onChange={(e) => update({ voice_speed: Number(e.target.value) })}
            />
            <output>{settings.voice_speed.toFixed(2)}×</output>
          </div>
        </label>
      </section>

      <section>
        <textarea
          value={text}
          placeholder="Type or paste a script here to hear it, or to save it as an audio file."
          aria-label="Script"
          spellCheck={false}
          onChange={(e) => {
            draft = e.target.value;
            setText(e.target.value);
          }}
        />
        <div className="actions">
          {working ? (
            <button className="primary" onClick={() => api.stopSpeaking()}>
              Stop
            </button>
          ) : (
            <button className="primary" disabled={empty} onClick={() => start(api.speakText)}>
              Play
            </button>
          )}
          <button disabled={empty || working} onClick={() => start(api.exportSpeech)}>
            Save as audio
          </button>
          {speech.state === "exporting" ? (
            <div className="download">
              <div className="bar">
                <div style={{ width: `${speech.progress * 100}%` }} />
              </div>
            </div>
          ) : result.error ? (
            <span className="error">{result.error}</span>
          ) : result.saved ? (
            <span className="hint">Saved to Downloads as {fileName(result.saved)}</span>
          ) : speech.state === "preparing" ? (
            <span className="hint">Getting the voice ready…</span>
          ) : null}
        </div>
      </section>
    </>
  );
}

function fileName(path: string) {
  return path.split(/[\\/]/).pop() ?? path;
}
