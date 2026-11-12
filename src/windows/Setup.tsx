// Pieces shared by the first-run flow and the settings window: permission
// rows and the model picker with its download progress bar.
import type { ReactNode } from "react";
import { api, type DownloadProgress, type ModelInfo, type SetupState } from "../lib/ipc";

/** A numbered "how to use it" strip. */
export function Steps({ steps }: { steps: ReactNode[] }) {
  return (
    <ol className="steps">
      {steps.map((step, i) => (
        <li key={i}>
          <span className="step-number">{i + 1}</span>
          <span>{step}</span>
        </li>
      ))}
    </ol>
  );
}

export function Check({ ok }: { ok: boolean }) {
  return <span className={ok ? "check ok" : "check"}>{ok ? "✓" : ""}</span>;
}

export function MicrophoneRow({ setup }: { setup: SetupState }) {
  const granted = setup.microphone === "granted";
  return (
    <div className="row">
      <Check ok={granted} />
      <div className="row-text">
        <strong>Microphone</strong>
        <span>
          {setup.microphone === "denied"
            ? "Access was denied. Turn Loudr on in system settings."
            : "Loudr listens only while you hold the hotkey."}
        </span>
      </div>
      {!granted &&
        (setup.microphone === "unknown" ? (
          <button onClick={() => api.requestMicrophone()}>Allow</button>
        ) : (
          <button onClick={() => api.openPrivacySettings("microphone")}>Open settings</button>
        ))}
    </div>
  );
}

export function AccessibilityRow({ setup }: { setup: SetupState }) {
  // Only macOS gates keyboard access.
  if (setup.platform !== "macos") return null;
  return (
    <div className="row">
      <Check ok={setup.accessibility} />
      <div className="row-text">
        <strong>Accessibility</strong>
        <span>Needed to notice the hotkey and to type the text into other apps.</span>
      </div>
      {!setup.accessibility && (
        <button
          onClick={async () => {
            // The request adds Loudr to the list; then open that list.
            await api.requestAccessibility();
            await api.openPrivacySettings("accessibility");
          }}
        >
          Open settings
        </button>
      )}
    </div>
  );
}

export function ModelSelect({
  models,
  value,
  onChange,
}: {
  models: ModelInfo[];
  value: string;
  onChange: (id: string) => void;
}) {
  return (
    <select value={value} onChange={(e) => onChange(e.target.value)}>
      {models.map((m) => (
        <option key={m.id} value={m.id}>
          {m.label} · {m.size_mb >= 1000 ? `${m.size_mb / 1000} GB` : `${m.size_mb} MB`}
          {m.downloaded ? " · downloaded" : ""}
        </option>
      ))}
    </select>
  );
}

export function DownloadBar({ progress }: { progress: DownloadProgress | null }) {
  if (!progress) return null;
  if (progress.error) return <p className="error">{progress.error}</p>;
  const mb = (bytes: number) => (bytes / 1_000_000).toFixed(0);
  const percent = progress.total ? (100 * progress.downloaded) / progress.total : 0;
  return (
    <div className="download">
      <div className="bar">
        <div style={{ width: `${percent}%` }} />
      </div>
      <span>
        {mb(progress.downloaded)} / {mb(progress.total)} MB
      </span>
    </div>
  );
}
