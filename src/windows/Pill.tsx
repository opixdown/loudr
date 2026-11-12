// The floating pill: waveform while recording, spinner while transcribing
// or getting ready to speak, moving bars while reading aloud, a short
// message on errors, and nothing at all when idle.
import { useEffect, useRef, useState } from "react";
import { on, type Status } from "../lib/ipc";

const BARS = 28;
const BAR_WIDTH = 3;
const BAR_GAP = 2;
const HEIGHT = 24;
const WIDTH = BARS * (BAR_WIDTH + BAR_GAP) - BAR_GAP;
const ERROR_VISIBLE_MS = 4000;

export default function Pill() {
  const [status, setStatus] = useState<Status>({ state: "idle" });
  const canvas = useRef<HTMLCanvasElement>(null);
  // Most recent loudness values, oldest first. Kept in a ref, not state:
  // the canvas is redrawn directly when a level arrives, so React does not
  // re-render 30 times a second and nothing animates while idle.
  const levels = useRef<number[]>(new Array(BARS).fill(0));

  useEffect(() => {
    let errorTimer: number | undefined;
    const offStatus = on<Status>("status", (next) => {
      window.clearTimeout(errorTimer);
      if (next.state === "recording") levels.current.fill(0);
      if (next.state === "error") {
        errorTimer = window.setTimeout(() => setStatus({ state: "idle" }), ERROR_VISIBLE_MS);
      }
      setStatus(next);
    });
    const offLevel = on<number>("level", (rms) => {
      // Speech RMS is small (around 0.02 to 0.2); the square root spreads
      // it over the bar height.
      levels.current.push(Math.min(1, Math.sqrt(rms) * 2.2));
      levels.current.shift();
      draw(canvas.current, levels.current);
    });
    return () => {
      window.clearTimeout(errorTimer);
      offStatus();
      offLevel();
    };
  }, []);

  // Draw the empty waveform as soon as the canvas appears.
  useEffect(() => {
    if (status.state === "recording") draw(canvas.current, levels.current);
  }, [status.state]);

  if (status.state === "idle") return null;
  return (
    <div className="pill-stage">
      <div className={`pill pill-${status.state}`}>
        {status.state === "recording" && (
          <canvas ref={canvas} style={{ width: WIDTH, height: HEIGHT }} />
        )}
        {(status.state === "transcribing" || status.state === "preparing") && (
          <div className="spinner" />
        )}
        {status.state === "speaking" && (
          <div className="speaking" aria-label="Reading aloud">
            <i /><i /><i /><i /><i />
          </div>
        )}
        {status.state === "error" && <span className="pill-message">{status.message}</span>}
      </div>
    </div>
  );
}

function draw(canvas: HTMLCanvasElement | null, levels: number[]) {
  const ctx = canvas?.getContext("2d");
  if (!canvas || !ctx) return;
  const scale = window.devicePixelRatio || 1;
  if (canvas.width !== WIDTH * scale) {
    canvas.width = WIDTH * scale;
    canvas.height = HEIGHT * scale;
  }
  ctx.setTransform(scale, 0, 0, scale, 0, 0);
  ctx.clearRect(0, 0, WIDTH, HEIGHT);
  // The bars take the pill's text colour, so they follow the theme.
  ctx.fillStyle = getComputedStyle(canvas).color;
  levels.forEach((level, i) => {
    const h = Math.max(BAR_WIDTH, level * HEIGHT);
    ctx.beginPath();
    ctx.roundRect(i * (BAR_WIDTH + BAR_GAP), (HEIGHT - h) / 2, BAR_WIDTH, h, BAR_WIDTH / 2);
    ctx.fill();
  });
}
