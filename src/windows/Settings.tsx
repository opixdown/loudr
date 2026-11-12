// Settings window. Every change is saved and applied immediately.
import { useEffect, useState } from "react";
import { api, on, type Settings as SettingsType, type SetupState } from "../lib/ipc";
import { useModels } from "../lib/hooks";
import { LANGUAGES, THEMES } from "../lib/options";
import { AccessibilityRow, DownloadBar, MicrophoneRow, ModelSelect, Steps } from "./Setup";
import Speech from "./Speech";

type Tab = "dictation" | "speech";

export default function Settings({ setup }: { setup: SetupState }) {
  const [settings, setSettings] = useState<SettingsType | null>(null);
  const [devices, setDevices] = useState<string[]>([]);
  const [tab, setTab] = useState<Tab>("dictation");
  // The tray menu and the read-aloud key can ask for a particular tab.
  useEffect(() => on<Tab>("navigate", setTab), []);
  const { models, progress } = useModels(
    `${settings?.model}:${settings?.language}:${setup.model_ready}`,
  );

  useEffect(() => {
    api.getSettings().then(setSettings);
    api.listInputDevices().then(setDevices);
    // The tray's model picker can change settings too.
    return on<SettingsType>("settings-changed", setSettings);
  }, []);
  if (!settings) return null;

  const update = (patch: Partial<SettingsType>) => {
    const next = { ...settings, ...patch };
    setSettings(next);
    api.setSettings(next);
  };

  const permissionsMissing = setup.microphone !== "granted" || !setup.accessibility;
  // A saved microphone that is unplugged right now still needs an entry.
  const deviceMissing = settings.input_device && !devices.includes(settings.input_device);

  return (
    <main>
      <h1>Loudr</h1>

      <div className="tabs" role="tablist">
        {(["dictation", "speech"] as const).map((id) => (
          <button
            key={id}
            role="tab"
            aria-selected={tab === id}
            className={tab === id ? "selected" : undefined}
            onClick={() => setTab(id)}
          >
            {id === "dictation" ? "Dictation" : "Text to speech"}
          </button>
        ))}
      </div>

      {permissionsMissing && (
        <section className="warning">
          <MicrophoneRow setup={setup} />
          <AccessibilityRow setup={setup} />
        </section>
      )}

      {tab === "speech" ? (
        <Speech setup={setup} settings={settings} update={update} />
      ) : (
      <section>
        <Steps
          steps={[
            "Click where you want to type, in any app.",
            <>
              Hold <kbd>{setup.hotkey_label}</kbd> and speak.
            </>,
            "Let go. Your words are typed in.",
          ]}
        />
        <KeyRow
          target="hold"
          title="Dictation key"
          hint="Keep it held down while you speak."
          label={setup.hotkey_label}
        />

        <label>
          <span>Model</span>
          <ModelSelect models={models} value={settings.model} onChange={(model) => update({ model })} />
        </label>
        {!setup.model_ready && !progress && !setup.downloading && (
          <div className="inline end">
            <span className="error">This model is not downloaded.</span>
            <button onClick={() => api.downloadModel()}>Download</button>
          </div>
        )}
        <DownloadBar progress={progress} />

        <label>
          <span>Language</span>
          <select value={settings.language} onChange={(e) => update({ language: e.target.value })}>
            {LANGUAGES.map(([code, label]) => (
              <option key={code} value={code}>
                {label}
              </option>
            ))}
          </select>
        </label>
        {settings.language !== "en" && (
          <p className="hint">Other languages use a separate multilingual model file.</p>
        )}

        <label>
          <span>Microphone</span>
          <select
            value={settings.input_device ?? ""}
            onFocus={() => api.listInputDevices().then(setDevices)}
            onChange={(e) => update({ input_device: e.target.value || null })}
          >
            <option value="">System default</option>
            {deviceMissing && (
              <option value={settings.input_device!}>{settings.input_device} (not connected)</option>
            )}
            {devices.map((name) => (
              <option key={name} value={name}>
                {name}
              </option>
            ))}
          </select>
        </label>

        <label>
          <span>Appearance</span>
          <select
            value={settings.theme}
            onChange={(e) => update({ theme: e.target.value as SettingsType["theme"] })}
          >
            {THEMES.map(([id, label]) => (
              <option key={id} value={id}>
                {label}
              </option>
            ))}
          </select>
        </label>

        <label>
          <span>Launch at login</span>
          <input
            type="checkbox"
            checked={settings.launch_at_login}
            onChange={(e) => update({ launch_at_login: e.target.checked })}
          />
        </label>
      </section>
      )}

      {tab === "dictation" && (
        <p className="hint">Loudr works offline. Audio and text never leave this computer.</p>
      )}
    </main>
  );
}

/** Shows one of the two global keys and lets the user record a different one. */
export function KeyRow({
  target,
  title,
  hint,
  label,
}: {
  target: "hold" | "speak";
  title: string;
  hint: string;
  label: string;
}) {
  const [capturing, setCapturing] = useState(false);
  const [rejected, setRejected] = useState(false);

  useEffect(() => {
    // Both key rows hear this event; only the one that is recording reacts.
    if (!capturing) return;
    return on<"captured" | "cancelled" | "unsupported">("hotkey-capture", (outcome) => {
      // An unusable key keeps the capture going; anything else ends it.
      setRejected(outcome === "unsupported");
      if (outcome !== "unsupported") setCapturing(false);
    });
  }, [capturing]);

  return (
    <div className="row hold-key">
      <div className="row-text">
        <strong>{title}</strong>
        <span className={rejected ? "error" : undefined}>
          {!capturing
            ? hint
            : rejected
              ? "That key can't be used. Press a modifier key (Option, Command, Control, Shift) or a function key that isn't already taken."
              : "Press the key you want to use. Esc cancels."}
        </span>
      </div>
      {capturing ? (
        <button
          onClick={() => {
            api.cancelHotkeyCapture();
            setCapturing(false);
            setRejected(false);
          }}
        >
          Cancel
        </button>
      ) : (
        <>
          <kbd>{label}</kbd>
          <button
            onClick={() => {
              setRejected(false);
              setCapturing(true);
              api.startHotkeyCapture(target);
            }}
          >
            Change
          </button>
        </>
      )}
    </div>
  );
}
