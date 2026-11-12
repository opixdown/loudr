// Scenes for the text-to-speech video. As in scenes.tsx, each is timed to
// its voiceover line and places animations at fractions of its length.
import React from "react";
import { Img, interpolate, spring, staticFile, useCurrentFrame, useVideoConfig } from "remotion";
import { Pill, Rise, Scene, SpeakingBars } from "./parts";
import { column, headline, sub } from "./scenes";
import { C } from "./theme";

type SceneProps = { duration: number };
const clamp = { extrapolateLeft: "clamp", extrapolateRight: "clamp" } as const;

/* 1. "Hi. I'm the voice inside Loudr." */
export const Intro: React.FC<SceneProps> = ({ duration }) => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const pop = spring({ frame: frame - 2, fps, config: { damping: 13, mass: 0.7 } });
  return (
    <Scene duration={duration}>
      <div style={column(48)}>
        <div style={{ display: "flex", alignItems: "center", gap: 44 }}>
          <Img
            src={staticFile("icon.png")}
            style={{ width: 220, height: 220, transform: `scale(${pop})` }}
          />
          <Rise at={4}>
            <Pill scale={1.7}>
              <SpeakingBars height={84} width={12} />
            </Pill>
          </Rise>
        </div>
        <Rise at={0.3 * duration}>
          <h1 style={headline}>I'm the voice inside Loudr.</h1>
        </Rise>
        <Rise at={0.55 * duration}>
          <p style={sub}>This narration was made by the app, on a laptop, offline.</p>
        </Rise>
      </div>
    </Scene>
  );
};

/* 2. "Loudr already types what you say. Now, it talks back." */
export const Turn: React.FC<SceneProps> = ({ duration }) => {
  const frame = useCurrentFrame();
  const second = 0.56 * duration;
  return (
    <Scene duration={duration}>
      <div style={{ ...column(12), alignItems: "flex-start" }}>
        <Rise>
          <h1
            style={{
              ...headline,
              fontSize: 120,
              textAlign: "left",
              color: frame < second ? C.text : C.track,
            }}
          >
            It types what you say.
          </h1>
        </Rise>
        <Rise at={second}>
          <h1 style={{ ...headline, fontSize: 170, textAlign: "left" }}>Now it talks back.</h1>
        </Rise>
      </div>
    </Scene>
  );
};

const ARTICLE = [
  "The best ideas rarely arrive at a desk.",
  "They show up on a walk, in the shower,",
  "or halfway through another sentence.",
];

/* 3. "Select text in any app. Tap one key. Then just listen." */
export const ReadAloud: React.FC<SceneProps> = ({ duration }) => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const at = (f: number) => Math.round(f * duration);
  const SELECT_START = at(0.04);
  const SELECT_END = at(0.27);
  const TAP = at(READ_TAP);
  const LISTEN = TAP + 7;
  const step = frame < SELECT_START ? -1 : frame < TAP ? 0 : frame < LISTEN ? 1 : 2;
  const steps = ["Select.", "Tap.", "Listen."];
  const pressed = frame >= TAP && frame < LISTEN;

  // The selection sweeps across the lines; later the same sweep, slower,
  // marks the words that have been read.
  const total = ARTICLE.join("").length;
  const selected = interpolate(frame, [SELECT_START, SELECT_END], [0, total], clamp);
  const read = interpolate(frame, [LISTEN + 4, duration - 12], [0, total], clamp);
  const pillIn = spring({ frame: frame - LISTEN, fps, config: { damping: 200 }, durationInFrames: 8 });

  let before = 0;
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
              <span style={{ marginLeft: 18, fontSize: 24, color: C.muted }}>Browser</span>
            </div>
            <div style={{ padding: "40px 56px", fontSize: 46, lineHeight: 1.5 }}>
              {ARTICLE.map((line) => {
                const start = before;
                before += line.length;
                const sel = Math.max(0, Math.min(line.length, selected - start));
                const done = Math.max(0, Math.min(line.length, read - start));
                return (
                  <div key={line} style={{ position: "relative", width: "fit-content" }}>
                    {/* selection highlight */}
                    <div
                      style={{
                        position: "absolute",
                        left: -6,
                        top: 6,
                        bottom: 6,
                        width: `calc(${(sel / line.length) * 100}% + ${sel > 0 ? 12 : 0}px)`,
                        background: C.panel,
                        borderRadius: 8,
                      }}
                    />
                    <span style={{ position: "relative", color: C.muted }}>
                      <span style={{ color: C.text }}>
                        {line.slice(0, step === 2 ? Math.round(done) : line.length)}
                      </span>
                      {step === 2 ? line.slice(Math.round(done)) : ""}
                    </span>
                  </div>
                );
              })}
            </div>
          </div>

          {step === 2 && (
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
              <Pill>
                <SpeakingBars />
              </Pill>
            </div>
          )}

          {/* The key being tapped */}
          <div
            style={{
              position: "absolute",
              right: -150,
              bottom: -30,
              width: 250,
              height: 150,
              borderRadius: 26,
              background: pressed ? C.ink : C.control,
              color: pressed ? C.onInk : C.text,
              border: `2px solid ${pressed ? C.ink : C.border}`,
              boxShadow: pressed ? "none" : `0 10px 0 ${C.border}`,
              transform: `translateY(${pressed ? 10 : 0}px)`,
              display: "flex",
              flexDirection: "column",
              alignItems: "flex-end",
              justifyContent: "space-between",
              padding: "18px 24px",
              boxSizing: "border-box",
              fontSize: 28,
            }}
          >
            <span style={{ fontSize: 44 }}>⌘</span>
            <span>right command</span>
          </div>
        </div>
      </div>
    </Scene>
  );
};
/** When the key goes down in ReadAloud, as a fraction of the scene. */
export const READ_TAP = 0.44;

const SCRIPT = "Welcome back to the channel. Today, three tools I can't work without.";

/* 4. "Or paste a script, pick a voice, and save it as an MP3." */
export const Script: React.FC<SceneProps> = ({ duration }) => {
  const frame = useCurrentFrame();
  const at = (f: number) => Math.round(f * duration);
  const pasted = frame >= at(0.1);
  const voicePicked = frame >= at(0.36);
  const savePressed = frame >= at(SCRIPT_SAVE) && frame < at(SCRIPT_SAVE) + 6;
  const saved = frame >= at(SCRIPT_SAVE) + 6;
  const button: React.CSSProperties = {
    fontSize: 32,
    fontWeight: 500,
    padding: "16px 34px",
    borderRadius: 999,
    border: `2px solid ${C.border}`,
    background: C.control,
  };
  return (
    <Scene duration={duration}>
      <div style={column(40)}>
        <Rise>
          <h1 style={headline}>Script in. MP3 out.</h1>
        </Rise>
        <div
          style={{
            width: 1180,
            background: C.panel,
            borderRadius: 36,
            padding: "36px 40px",
            display: "flex",
            flexDirection: "column",
            gap: 24,
          }}
        >
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
            <span style={{ fontSize: 34, fontWeight: 600 }}>Script to audio</span>
            <span
              style={{
                ...button,
                fontSize: 28,
                padding: "12px 28px",
                borderColor: voicePicked && frame < at(0.5) ? C.ink : C.border,
              }}
            >
              Voice · {voicePicked ? "Puck, American" : "Heart, American"} ▾
            </span>
          </div>
          <div
            style={{
              background: C.control,
              border: `2px solid ${C.border}`,
              borderRadius: 24,
              padding: "26px 30px",
              height: 170,
              fontSize: 38,
              lineHeight: 1.4,
              color: pasted ? C.text : C.muted,
            }}
          >
            {pasted ? SCRIPT : "Type or paste a script."}
          </div>
          <div style={{ display: "flex", gap: 16, alignItems: "center" }}>
            <span style={{ ...button, background: C.ink, color: C.onInk, borderColor: C.ink }}>Play</span>
            <span
              style={{
                ...button,
                background: savePressed ? C.ink : C.control,
                color: savePressed ? C.onInk : C.text,
                borderColor: savePressed ? C.ink : C.border,
              }}
            >
              Save as audio…
            </span>
            {saved && (
              <Rise at={at(SCRIPT_SAVE) + 6} distance={14} style={{ marginLeft: "auto" }}>
                <span
                  style={{
                    display: "flex",
                    alignItems: "center",
                    gap: 14,
                    fontSize: 30,
                    fontWeight: 600,
                    padding: "14px 28px",
                    borderRadius: 18,
                    background: C.control,
                    border: `2px solid ${C.ink}`,
                  }}
                >
                  ♪ Loudr - Welcome back.mp3
                </span>
              </Rise>
            )}
          </div>
        </div>
      </div>
    </Scene>
  );
};
/** When Save is pressed in Script, as a fraction of the scene. */
export const SCRIPT_SAVE = 0.7;
