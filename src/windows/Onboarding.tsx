// First-run flow: permissions, then the model download.
import { useEffect, useState } from "react";
import { api, type Settings, type SetupState } from "../lib/ipc";
import { useModels } from "../lib/hooks";
import { AccessibilityRow, Check, DownloadBar, MicrophoneRow, ModelSelect } from "./Setup";

export default function Onboarding({ setup, onDone }: { setup: SetupState; onDone: () => void }) {
  const [settings, setSettings] = useState<Settings | null>(null);
  const { models, progress, clearProgress } = useModels(
    `${settings?.model}:${setup.model_ready}`,
  );
  useEffect(() => {
    api.getSettings().then(setSettings);
  }, []);
  if (!settings) return null;

  const busy = setup.downloading || (progress !== null && !progress.error);
  const ready = setup.microphone === "granted" && setup.accessibility && setup.model_ready;

  // Changing the model here must not start a download by itself, so only the
  // local copy changes until the user presses Download.
  const download = async () => {
    clearProgress();
    await api.setSettings(settings); // starts the download if it's a new model
    if (!setup.downloading) await api.downloadModel().catch(() => {});
  };

  return (
    <main>
      <h1>Welcome to Loudr</h1>
      <p className="lead">
        Hold a key, speak, release. Your words are typed into whatever app you are using.
        Everything runs on this computer.
      </p>

      <section>
        <MicrophoneRow setup={setup} />
        <AccessibilityRow setup={setup} />
        <div className="row">
          <Check ok={setup.model_ready} />
          <div className="row-text">
            <strong>Speech model</strong>
            <span>Downloaded once, then Loudr never goes online again.</span>
            {!setup.model_ready && (
              <div className="inline">
                <ModelSelect
                  models={models}
                  value={settings.model}
                  onChange={(model) => setSettings({ ...settings, model })}
                />
                <button className="primary" disabled={busy} onClick={download}>
                  {busy ? "Downloading…" : "Download"}
                </button>
              </div>
            )}
            <DownloadBar progress={progress} />
          </div>
        </div>
      </section>

      <button
        className="primary wide"
        disabled={!ready}
        onClick={async () => {
          await api.finishOnboarding();
          onDone();
        }}
      >
        {ready ? "Start using Loudr" : "Finish the steps above to continue"}
      </button>
      {ready && (
        <p className="hint">
          Hold <kbd>{setup.hotkey_label}</kbd> anywhere and speak.
        </p>
      )}
    </main>
  );
}
