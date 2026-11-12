// The eight scenes. Each one is timed to its voiceover line, so animations
// are placed with `at(fraction)`: a fraction of the scene's length.
import React from "react";
import { Img, interpolate, spring, staticFile, useCurrentFrame, useVideoConfig } from "remotion";
import { Pill, Rise, Scene, Spinner, Waveform } from "./parts";
import { C } from "./theme";

type SceneProps = { duration: number };
const clamp = { extrapolateLeft: "clamp", extrapolateRight: "clamp" } as const;

export const headline: React.CSSProperties = {
  fontSize: 104,
  fontWeight: 600,
  letterSpacing: "-0.035em",
  lineHeight: 1.05,
  textAlign: "center",
  margin: 0,
};
export const sub: React.CSSProperties = {
  fontSize: 42,
  color: C.muted,
  textAlign: "center",
  margin: 0,
  letterSpacing: "-0.01em",
};
export const column = (gap: number): React.CSSProperties => ({
  display: "flex",
  flexDirection: "column",
  alignItems: "center",
  gap,
});

/* 1. "You speak at 150 words a minute. You type at 40." */
export const Hook: React.FC<SceneProps> = ({ duration }) => {
  const frame = useCurrentFrame();
  const rows = [
    { label: "Speaking", value: 150, start: 0.04, strong: true },
    { label: "Typing", value: 40, start: 0.6, strong: false },
  ];
  return (
    <Scene duration={duration}>
      <div style={{ ...column(40), alignItems: "stretch", width: 1400 }}>
        {rows.map((row) => {
          const from = row.start * duration;
          const grow = interpolate(frame, [from, from + 22], [0, 1], clamp);
          const eased = 1 - Math.pow(1 - grow, 3);
          return (
            <Rise key={row.label} at={from}>
              <div style={{ display: "flex", alignItems: "baseline", gap: 28 }}>
                <span
                  style={{
                    fontSize: 190,
                    fontWeight: 700,
                    letterSpacing: "-0.05em",
                    lineHeight: 1,
                    width: 360,
                    textAlign: "right",
                    fontVariantNumeric: "tabular-nums",
                    color: row.strong ? C.text : C.muted,
                  }}
                >
                  {Math.round(eased * row.value)}
                </span>
                <div style={{ flex: 1, display: "flex", flexDirection: "column", gap: 16 }}>
                  <span style={{ fontSize: 44, color: C.muted }}>
                    {row.label} · words per minute
                  </span>
                  <div
                    style={{
                      height: 36,
                      borderRadius: 18,
                      width: `${(eased * row.value) / 1.5}%`,
                      minWidth: 36,
                      background: row.strong ? C.ink : C.track,
                    }}
                  />
                </div>
              </div>
            </Rise>
          );
        })}
      </div>
    </Scene>
  );
};

/* 2. "Wispr Flow gets that. But its free plan runs out, and its code is closed." */
export const Problem: React.FC<SceneProps> = ({ duration }) => {
  const frame = useCurrentFrame();
  const at = (f: number) => f * duration;
  const fill = interpolate(frame, [at(0.3), at(0.56)], [0.4, 1], clamp);
  const full = frame >= at(0.56);
  return (
    <Scene duration={duration}>
      <div style={column(52)}>
        <Rise>
          <h1 style={headline}>
            Wispr Flow gets that.
            {frame >= at(0.28) && (
              <>
                <br />
                <span style={{ color: C.muted }}>But&hellip;</span>
              </>
            )}
          </h1>
        </Rise>

        <div style={{ display: "flex", gap: 28, alignItems: "stretch" }}>
          <Rise at={at(0.3)}>
            <div
              style={{
                width: 760,
                background: C.panel,
                borderRadius: 32,
                padding: "34px 40px",
                display: "flex",
                flexDirection: "column",
                gap: 22,
              }}
            >
              <div style={{ display: "flex", justifyContent: "space-between", fontSize: 34 }}>
                <span style={{ fontWeight: 600 }}>Free plan</span>
                <span style={{ color: full ? C.danger : C.muted, fontWeight: full ? 600 : 400 }}>
                  {full ? "Limit reached" : `${Math.round(fill * 100)}% used`}
                </span>
              </div>
              <div style={{ height: 18, borderRadius: 9, background: C.track, overflow: "hidden" }}>
                <div
                  style={{
                    width: `${fill * 100}%`,
                    height: "100%",
                    borderRadius: 9,
                    background: full ? C.danger : C.ink,
                  }}
                />
              </div>
            </div>
          </Rise>
          <Rise at={at(0.72)}>
            <div
              style={{
                height: "100%",
                boxSizing: "border-box",
                background: C.panel,
                borderRadius: 32,
                padding: "34px 44px",
                display: "flex",
                alignItems: "center",
                gap: 22,
                fontSize: 34,
                fontWeight: 600,
              }}
            >
              <Lock />
              Closed source
            </div>
          </Rise>
        </div>
      </div>
    </Scene>
  );
};

const Lock: React.FC = () => (
  <svg width="44" height="52" viewBox="0 0 44 52" fill="none">
    <path d="M11 22v-8a11 11 0 0 1 22 0v8" stroke={C.ink} strokeWidth="5" strokeLinecap="round" />
    <rect x="2" y="22" width="40" height="28" rx="8" fill={C.ink} />
  </svg>
);

/* 3. "So I built Loudr." */
export const Reveal: React.FC<SceneProps> = ({ duration }) => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const pop = spring({ frame: frame - 6, fps, config: { damping: 13, mass: 0.7 } });
  return (
    <Scene duration={duration}>
      <div style={column(20)}>
        <Rise>
          <p style={{ ...sub, fontSize: 52 }}>So I built</p>
        </Rise>
        <div style={{ display: "flex", alignItems: "center", gap: 36 }}>
          <Img
            src={staticFile("icon.png")}
            style={{ width: 250, height: 250, transform: `scale(${pop})` }}
          />
          <Rise at={10}>
            <h1 style={{ ...headline, fontSize: 210 }}>Loudr</h1>
          </Rise>
        </div>
      </div>
    </Scene>
  );
};

/* 4. "Hold a key. Speak. Release. Your words land in any app, in under a second." */
export const Demo: React.FC<SceneProps> = ({ duration }) => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const at = (f: number) => Math.round(f * duration);
  const HOLD_START = at(0.03);
  const SPEAK = at(0.17);
  const HOLD_END = at(0.3);
  const TEXT_AT = HOLD_END + 10; // about a third of a second after release
  const holding = frame >= HOLD_START && frame < HOLD_END;
  const transcribing = frame >= HOLD_END && frame < TEXT_AT;
  const pillIn = spring({
    frame: frame - HOLD_START,
    fps,
    config: { damping: 200 },
    durationInFrames: 8,
  });
  const step = frame < HOLD_START ? -1 : frame < SPEAK ? 0 : frame < HOLD_END ? 1 : 2;
  const steps = ["Hold a key.", "Speak.", "Release."];
  const cursorOn = Math.floor(frame / 16) % 2 === 0;

  return (
    <Scene duration={duration}>
      <div style={column(44)}>
        <div style={{ display: "flex", gap: 36 }}>
          {steps.map((label, i) => (
            <span key={label} style={{ ...headline, fontSize: 84, color: i <= step ? C.text : C.track }}>
              {label}
            </span>
          ))}
        </div>

        {/* A plain document window */}
        <div style={{ position: "relative" }}>
          <div
            style={{
              width: 1240,
              height: 500,
              borderRadius: 28,
              background: C.control,
              border: `2px solid ${C.border}`,
              overflow: "hidden",
            }}
          >
            <div
              style={{
                height: 64,
                borderBottom: `2px solid ${C.border}`,
                display: "flex",
                alignItems: "center",
                gap: 12,
                padding: "0 26px",
              }}
            >
              {[0, 1, 2].map((i) => (
                <div key={i} style={{ width: 18, height: 18, borderRadius: 9, background: C.track }} />
              ))}
              <span style={{ marginLeft: 18, fontSize: 24, color: C.muted }}>Notes</span>
            </div>
            <div style={{ padding: "44px 56px", fontSize: 46, lineHeight: 1.4 }}>
              {frame >= TEXT_AT && <span>Loudr types what I say, right where my cursor is. </span>}
              <span
                style={{
                  display: "inline-block",
                  width: 4,
                  height: 52,
                  verticalAlign: "-8px",
                  background: C.ink,
                  opacity: cursorOn ? 1 : 0,
                }}
              />
            </div>
          </div>

          {(holding || transcribing) && (
            <div
              style={{
                position: "absolute",
                left: 0,
                right: 0,
                bottom: 36,
                display: "flex",
                justifyContent: "center",
                opacity: pillIn,
                transform: `translateY(${(1 - pillIn) * 10}px)`,
              }}
            >
              <Pill>{holding ? <Waveform /> : <Spinner />}</Pill>
            </div>
          )}

          {/* The key being held */}
          <div
            style={{
              position: "absolute",
              right: -150,
              bottom: -30,
              width: 220,
              height: 150,
              borderRadius: 26,
              background: holding ? C.ink : C.control,
              color: holding ? C.onInk : C.text,
              border: `2px solid ${holding ? C.ink : C.border}`,
              boxShadow: holding ? "none" : `0 10px 0 ${C.border}`,
              transform: `translateY(${holding ? 10 : 0}px)`,
              display: "flex",
              flexDirection: "column",
              alignItems: "flex-end",
              justifyContent: "space-between",
              padding: "18px 24px",
              boxSizing: "border-box",
              fontSize: 28,
            }}
          >
            <span style={{ fontSize: 44 }}>⌥</span>
            <span>right option</span>
          </div>
        </div>

        <Rise at={at(0.55)}>
          <p style={sub}>
            Any app. <span style={{ color: C.text, fontWeight: 600 }}>Under a second.</span>
          </p>
        </Rise>
      </div>
    </Scene>
  );
};

/* 5. "It runs fully offline. Your voice never leaves your computer." */
export const Offline: React.FC<SceneProps & { title?: string; line?: string }> = ({
  duration,
  title = "Fully offline.",
  line = "Your voice never leaves your computer.",
}) => {
  const frame = useCurrentFrame();
  const slash = interpolate(frame, [8, 20], [0, 1], clamp);
  return (
    <Scene duration={duration}>
      <div style={column(40)}>
        {/* Wi-Fi symbol being struck through */}
        <svg width="260" height="210" viewBox="0 0 260 210" fill="none">
          {[100, 68, 36].map((r) => (
            <path
              key={r}
              d={`M ${130 - r * 1.1} ${160 - r * 0.9} A ${r * 1.45} ${r * 1.45} 0 0 1 ${130 + r * 1.1} ${160 - r * 0.9}`}
              stroke={C.track}
              strokeWidth="20"
              strokeLinecap="round"
            />
          ))}
          <circle cx="130" cy="170" r="16" fill={C.track} />
          <line
            x1="40"
            y1="20"
            x2={40 + 180 * slash}
            y2={20 + 180 * slash}
            stroke={C.ink}
            strokeWidth="20"
            strokeLinecap="round"
            opacity={slash > 0 ? 1 : 0}
          />
        </svg>
        <Rise at={4}>
          <h1 style={headline}>{title}</h1>
        </Rise>
        <Rise at={0.42 * duration}>
          <p style={sub}>{line}</p>
        </Rise>
      </div>
    </Scene>
  );
};

/* 6. "No account. No limits. No subscription." */
const FREE_LINES: [string, number][] = [
  ["No account.", 0.02],
  ["No limits.", 0.36],
  ["No subscription.", 0.62],
];
export const Free: React.FC<SceneProps & { lines?: [string, number][] }> = ({
  duration,
  lines = FREE_LINES,
}) => {
  const frame = useCurrentFrame();
  return (
    <Scene duration={duration}>
      <div style={{ ...column(8), alignItems: "flex-start" }}>
        {lines.map(([text, start], i) => {
          const next = lines[i + 1]?.[1] ?? 2;
          // The line being spoken is black; earlier ones step back to grey.
          const current = frame < next * duration;
          return (
            <Rise key={text} at={start * duration}>
              <h1
                style={{
                  ...headline,
                  fontSize: 150,
                  textAlign: "left",
                  color: current ? C.text : C.track,
                }}
              >
                {text}
              </h1>
            </Rise>
          );
        })}
      </div>
    </Scene>
  );
};

/* 7. "And it's open source. Want a feature? Add it." */
export const OpenSource: React.FC<SceneProps> = ({ duration }) => {
  const frame = useCurrentFrame();
  const command = "git clone https://github.com/opixdown/loudr";
  const typed = command.slice(0, Math.round(interpolate(frame, [6, 0.4 * duration], [0, command.length], clamp)));
  const merged = frame >= 0.78 * duration;
  return (
    <Scene duration={duration}>
      <div style={column(44)}>
        <Rise>
          <h1 style={headline}>Open source.</h1>
        </Rise>
        <div style={{ ...column(20), alignItems: "stretch", width: 1180 }}>
          <div
            style={{
              background: C.ink,
              color: C.onInk,
              borderRadius: 28,
              padding: "34px 40px",
              fontFamily: '"SF Mono", Menlo, Consolas, monospace',
              fontSize: 38,
            }}
          >
            <span style={{ opacity: 0.5 }}>$ </span>
            {typed}
            <span style={{ opacity: Math.floor(frame / 8) % 2 ? 0 : 1 }}>▍</span>
          </div>
          <Rise at={0.48 * duration}>
            <div
              style={{
                background: C.panel,
                borderRadius: 28,
                padding: "30px 40px",
                display: "flex",
                alignItems: "center",
                justifyContent: "space-between",
                fontSize: 38,
              }}
            >
              <span>
                <span style={{ color: C.muted }}>Pull request · </span>
                <span style={{ fontWeight: 600 }}>Add the feature you want</span>
              </span>
              <span
                style={{
                  fontSize: 30,
                  fontWeight: 600,
                  padding: "10px 26px",
                  borderRadius: 999,
                  background: merged ? C.ink : "transparent",
                  color: merged ? C.onInk : C.muted,
                  border: `2px solid ${merged ? C.ink : C.border}`,
                }}
              >
                {merged ? "✓ Merged" : "Open"}
              </span>
            </div>
          </Rise>
        </div>
      </div>
    </Scene>
  );
};

/* 8. "Loudr. Free, on GitHub." */
export const Outro: React.FC<
  SceneProps & { line?: string }
> = ({ line = "Free, offline, open-source voice typing. macOS and Windows." }) => (
  // No fade-out: the video ends on this frame.
  <Scene duration={100000}>
    <div style={column(30)}>
      <Rise>
        <Img src={staticFile("icon.png")} style={{ width: 230, height: 230 }} />
      </Rise>
      <Rise at={4}>
        <h1 style={{ ...headline, fontSize: 130 }}>Loudr</h1>
      </Rise>
      <Rise at={12}>
        <p style={sub}>{line}</p>
      </Rise>
      <Rise at={20} style={{ marginTop: 20 }}>
        <div
          style={{
            fontSize: 44,
            fontWeight: 600,
            padding: "24px 52px",
            borderRadius: 999,
            background: C.ink,
            color: C.onInk,
          }}
        >
          github.com/opixdown/loudr
        </div>
      </Rise>
    </div>
  </Scene>
);
