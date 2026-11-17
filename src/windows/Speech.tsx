// Text to speech tab: the read-aloud key, the voice, a box for turning a
// script into speech or an audio file, and the optional online voices.
import { useEffect, useState } from "react";
import {
  api,
  on,
  type DownloadProgress,
  type Settings,
  type SetupState,
  type Choice,
  type SpeechEvent,
  type StudioProvider,
  type Voices,
} from "../lib/ipc";
import { KeyRow } from "./Settings";
import { DownloadBar, Steps } from "./Setup";

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
  // Start of the text that was read aloud last (its audio can be saved).
  const [last, setLast] = useState<string | null>(null);
  const [providers, setProviders] = useState<StudioProvider[]>([]);
  const refreshProviders = () => api.studioProviders().then(setProviders);

  useEffect(() => {
    refreshProviders();
    api.listVoices().then(setVoices);
    api.lastSpeech().then(setLast);
    const offDownload = on<DownloadProgress>("voice-download", (p) =>
      setDownload(p.done ? null : p),
    );
    const offSpeech = on<SpeechEvent>("speech", (event) => {
      setSpeech(event);
      if (event.last !== undefined) setLast(event.last);
      if (event.error) setResult({ error: event.error });
      else if (event.saved) setResult({ saved: event.saved });
    });
    return () => {
      offDownload();
      offSpeech();
    };
  }, []);
  if (!voices) return null;

  // The online voice chosen for scripts, once it has a key.
  const studio = providers.find(
    (p) => p.id === settings.studio_provider && p.has_key,
  );
  const busy = setup.downloading || (download !== null && !download.error);
  const intro = (
    <section className="intro">
      <h2>Give Loudr a voice</h2>
      <p>
        A natural voice that runs on this computer. Nothing you read is sent
        anywhere.
      </p>
      <ul>
        <li>
          <strong>Read aloud.</strong> Select text in any app and tap a key to
          hear it.
        </li>
        <li>
          <strong>Script to audio.</strong> Paste a script and save it as an
          MP3.
        </li>
        <li>
          <strong>Thirteen voices.</strong> American, British and Hindi, with a
          speed control.
        </li>
      </ul>
      <button
        className="primary wide"
        disabled={busy}
        onClick={() => {
          setDownload(null);
          api.downloadVoice().catch(() => {});
        }}
      >
        {busy ? "Downloading…" : `Download voice · ${voices.download_mb} MB`}
      </button>
      <DownloadBar progress={download} />
      {!busy && !download?.error && (
        <p className="hint">One time only. Takes a minute or two.</p>
      )}
    </section>
  );

  const working = speech.state !== "idle";
  const empty = text.trim() === "";
  const start = (action: (text: string) => Promise<void>) => {
    setResult({});
    action(text);
  };

  return (
    <>
      {!setup.voice_ready && intro}
      {setup.voice_ready && (
        <section>
          <Steps
            steps={[
              "Select text in any app.",
              <>
                Tap <kbd>{setup.speak_label}</kbd>
              </>,
              "Listen. Tap again to stop.",
            ]}
          />
          <KeyRow
            target="speak"
            title="Read-aloud key"
            hint="A quick tap. Don't hold it."
            label={setup.speak_label}
          />

          <label>
            <span>Voice</span>
            <select
              value={settings.voice}
              onChange={(e) => update({ voice: e.target.value })}
            >
              {voices.voices.map((v) => (
                <option key={v.id} value={v.id}>
                  {v.label}
                </option>
              ))}
            </select>
          </label>
          {settings.voice.startsWith("h") && (
            <p className="hint">
              Reads Hindi, Hinglish (Hindi typed in English letters) and
              English.
            </p>
          )}

          <label>
            <span>Speed</span>
            <div className="speed">
              <input
                type="range"
                min={0.7}
                max={1.5}
                step={0.05}
                value={settings.voice_speed}
                onChange={(e) =>
                  update({ voice_speed: Number(e.target.value) })
                }
              />
              <output>{settings.voice_speed.toFixed(2)}×</output>
            </div>
          </label>
        </section>
      )}

      {(setup.voice_ready || studio) && (
        <section className="composer">
          <strong>Script to audio</strong>
          <textarea
            value={text}
            placeholder="Type or paste a script. Play it, or save it as an MP3."
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
              <button
                className="primary"
                disabled={empty}
                onClick={() => start(api.speakText)}
              >
                Play
              </button>
            )}
            <button
              disabled={empty || working}
              onClick={() => start(api.exportSpeech)}
            >
              Save as audio…
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
              <span className="hint">Saved as {fileName(result.saved)}</span>
            ) : speech.state === "preparing" ? (
              <span className="hint">
                {studio ? `Asking ${studio.name}…` : "Getting the voice ready…"}
              </span>
            ) : null}
          </div>
          {studio && (
            <p className="hint">
              Spoken by {studio.name}. Your script is sent to {studio.name} and
              uses your account's credit.
            </p>
          )}
        </section>
      )}

      <StudioVoice
        settings={settings}
        update={update}
        providers={providers}
        refresh={refreshProviders}
      />

      {last && (
        <section>
          <div className="row">
            <div className="row-text">
              <strong>Last read aloud</strong>
              <span className="clip">{last}</span>
            </div>
            <button
              disabled={working}
              onClick={() => {
                setResult({});
                api.saveLastSpeech();
              }}
            >
              Save audio…
            </button>
          </div>
        </section>
      )}
    </>
  );
}

const KEY_PAGES = {
  elevenlabs: "elevenlabs.io/app/settings/api-keys",
  gemini: "aistudio.google.com/apikey",
};

/** Connect an online voice provider with the user's own API key. */
function StudioVoice({
  settings,
  update,
  providers,
  refresh,
}: {
  settings: Settings;
  update: (patch: Partial<Settings>) => void;
  providers: StudioProvider[];
  refresh: () => Promise<void>;
}) {
  const provider = providers.find((p) => p.id === settings.studio_provider);
  const [key, setKey] = useState("");
  const [voices, setVoices] = useState<Choice[]>([]);
  const [problem, setProblem] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const voiceField =
    provider?.id === "gemini" ? "gemini_voice" : "elevenlabs_voice";
  const modelField =
    provider?.id === "gemini" ? "gemini_model" : "elevenlabs_model";

  // Load the voices this key can use, and pick the first if none is chosen.
  useEffect(() => {
    setVoices([]);
    setProblem(null);
    if (!provider?.has_key) return;
    let current = true;
    setLoading(true);
    api
      .listStudioVoices(provider.id)
      .then((list) => {
        if (!current) return;
        setVoices(list);
        if (!list.some((v) => v.id === settings[voiceField])) {
          update({ [voiceField]: list[0]?.id ?? "" });
        }
      })
      .catch((e) => current && setProblem(String(e)))
      .finally(() => current && setLoading(false));
    return () => {
      current = false;
    };
  }, [provider?.id, provider?.has_key]);

  const saveKey = (value: string) => {
    if (!provider) return;
    setProblem(null);
    api
      .setStudioKey(provider.id, value)
      .then(() => {
        setKey("");
        return refresh();
      })
      .catch((e) => setProblem(String(e)));
  };

  return (
    <section>
      <label>
        <span>Studio voice</span>
        <select
          value={settings.studio_provider}
          onChange={(e) => {
            setKey("");
            update({
              studio_provider: e.target.value as Settings["studio_provider"],
            });
          }}
        >
          <option value="">Off</option>
          {providers.map((p) => (
            <option key={p.id} value={p.id}>
              {p.name}
            </option>
          ))}
        </select>
      </label>
      {!provider && (
        <p className="hint">
          Optional. Connect your own ElevenLabs or Google Gemini account to make
          scripts sound like a real person. This part works online; everything
          else stays on your computer.
        </p>
      )}

      {provider && !provider.has_key && (
        <>
          <div className="inline key">
            <input
              type="password"
              value={key}
              placeholder={`${provider.name} API key`}
              aria-label={`${provider.name} API key`}
              autoComplete="off"
              spellCheck={false}
              onChange={(e) => setKey(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && key.trim() && saveKey(key)}
            />
            <button
              className="primary"
              disabled={!key.trim()}
              onClick={() => saveKey(key)}
            >
              Save key
            </button>
          </div>
          <p className="hint">
            Create a key at {KEY_PAGES[provider.id]}. It is stored only on this
            computer.
          </p>
        </>
      )}

      {provider?.has_key && (
        <>
          <label>
            <span>Voice</span>
            <select
              value={settings[voiceField]}
              disabled={voices.length === 0}
              onChange={(e) => update({ [voiceField]: e.target.value })}
            >
              {voices.length === 0 && (
                <option>{loading ? "Loading…" : "No voices"}</option>
              )}
              {voices.map((v) => (
                <option key={v.id} value={v.id}>
                  {v.label}
                </option>
              ))}
            </select>
          </label>
          <label>
            <span>Quality</span>
            <select
              value={settings[modelField] || provider.models[0].id}
              onChange={(e) => update({ [modelField]: e.target.value })}
            >
              {provider.models.map((m) => (
                <option key={m.id} value={m.id}>
                  {m.label}
                </option>
              ))}
            </select>
          </label>
          <div className="inline end">
            <span className="hint grow">
              Used for Script to audio. Reading a selection aloud stays offline.
            </span>
            <button onClick={() => saveKey("")}>Remove key</button>
          </div>
        </>
      )}
      {problem && <p className="error">{problem}</p>}
    </section>
  );
}

function fileName(path: string) {
  return path.split(/[\\/]/).pop() ?? path;
}
