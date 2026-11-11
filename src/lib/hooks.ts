import { useCallback, useEffect, useState } from "react";
import { api, on, type DownloadProgress, type ModelInfo, type SetupState } from "./ipc";

/** Permission and model status, refreshed every second while the window is open. */
export function useSetupState(): SetupState | null {
  const [state, setState] = useState<SetupState | null>(null);
  useEffect(() => {
    const refresh = () => api.getSetupState().then(setState);
    refresh();
    const timer = setInterval(refresh, 1000);
    return () => clearInterval(timer);
  }, []);
  return state;
}

/** Model list plus live download progress. */
export function useModels(refreshKey: string) {
  const [models, setModels] = useState<ModelInfo[]>([]);
  const [progress, setProgress] = useState<DownloadProgress | null>(null);

  const refresh = useCallback(() => api.listModels().then(setModels), []);
  useEffect(() => {
    refresh();
  }, [refresh, refreshKey]);

  useEffect(
    () =>
      on<DownloadProgress>("model-download", (p) => {
        // Keep an error on screen; clear the bar when a download completes.
        setProgress(p.done ? null : p);
        if (p.done || p.error) refresh();
      }),
    [refresh],
  );

  return { models, progress, clearProgress: () => setProgress(null) };
}
