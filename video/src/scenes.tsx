import React from "react";
import { Img, interpolate, spring, staticFile, useCurrentFrame, useVideoConfig } from "remotion";
import { Pill, Rise, Scene, Spinner, Waveform } from "./parts";
import { C } from "./theme";

const clamp = { extrapolateLeft: "clamp", extrapolateRight: "clamp" } as const;
const headline: React.CSSProperties = {
  fontSize: 92,
  fontWeight: 600,
  letterSpacing: "-0.03em",
  lineHeight: 1.08,
  textAlign: "center",
  margin: 0,
};
const sub: React.CSSProperties = {
  fontSize: 40,
  color: C.muted,
  textAlign: "center",
  margin: 0,
  letterSpacing: "-0.01em",
};

/* 1. Voice typing is great. */
export const Hook: React.FC<{ duration: number }> = ({ duration }) => (
  <Scene duration={duration}>
    <div style={{ display: "flex", flexDirection: "column", alignItems: "center", gap: 56 }}>
      <Rise>
        <Pill scale={1.4}>
          <Waveform bars={30} barWidth={9} gap={6} height={72} />
        </Pill>
      </Rise>
      <Rise at={10}>
        <h1 style={headline}>
          Wispr Flow made voice typing
          <br />
          feel like magic.
        </h1>
      </Rise>
    </div>
  </Scene>
);

/* 2. But the free plan only goes so far. */
export const Problem: React.FC<{ duration: number }> = ({ duration }) => {
  const frame = useCurrentFrame();
  const fill = interpolate(frame, [18, 70], [0.35, 1], clamp);
  const full = frame >= 70;
  const chips = ["Usage limits", "Paid plans", "Closed source"];
  return (
    <Scene duration={duration}>
      <div style={{ display: "flex", flexDirection: "column", alignItems: "center", gap: 52 }}>
        <Rise>
          <h1 style={headline}>
            It&rsquo;s free.
            <br />
            <span style={{ color: C.muted }}>But not all of it.</span>
          </h1>
        </Rise>

        {/* Free plan meter filling up */}
        <Rise at={12} style={{ width: 860 }}>
          <div
            style={{
              background: C.panel,
              borderRadius: 32,
              padding: "32px 40px",
              display: "flex",
              flexDirection: "column",
              gap: 20,
            }}
          >
            <div style={{ display: "flex", justifyContent: "space-between", fontSize: 30 }}>
              <span style={{ fontWeight: 600 }}>Free plan</span>
              <span style={{ color: full ? C.danger : C.muted, fontWeight: full ? 600 : 400 }}>
                {full ? "Limit reached" : `${Math.round(fill * 100)}% used`}
              </span>
            </div>
            <div style={{ height: 16, borderRadius: 8, background: C.track, overflow: "hidden" }}>
              <div
                style={{
                  width: `${fill * 100}%`,
                  height: "100%",
                  borderRadius: 8,
                  background: full ? C.danger : C.ink,
                }}
              />
            </div>
          </div>
        </Rise>

        <div style={{ display: "flex", gap: 20 }}>
          {chips.map((chip, i) => (
            <Rise key={chip} at={82 + i * 9}>
              <div
                style={{
                  fontSize: 30,
                  padding: "14px 30px",
                  borderRadius: 999,
                  border: `2px solid ${C.border}`,
                  background: C.control,
                }}
              >
                {chip}
              </div>
            </Rise>
          ))}
        </div>
      </div>
    </Scene>
  );
};

/* 3. Meet Loudr. */
export const Reveal: React.FC<{ duration: number }> = ({ duration }) => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const pop = spring({ frame, fps, config: { damping: 14, mass: 0.8 } });
  return (
    <Scene duration={duration}>
      <div style={{ display: "flex", flexDirection: "column", alignItems: "center", gap: 28 }}>
        <Img
          src={staticFile("icon.png")}
          style={{ width: 300, height: 300, transform: `scale(${pop})` }}
        />
        <Rise at={12}>
          <h1 style={{ ...headline, fontSize: 132 }}>Loudr</h1>
        </Rise>
        <Rise at={22}>
          <p style={sub}>The open-source alternative. Free, all of it.</p>
        </Rise>
      </div>
    </Scene>
  );
};

/* 4. Hold a key, speak, release. */
export const Demo: React.FC<{ duration: number }> = ({ duration }) => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const HOLD_START = 30;
  const HOLD_END = 118;
  const TEXT_AT = HOLD_END + 12; // about 0.4 s after release
  const holding = frame >= HOLD_START && frame < HOLD_END;
  const transcribing = frame >= HOLD_END && frame < TEXT_AT;
  const pillIn = spring({ frame: frame - HOLD_START, fps, config: { damping: 200 }, durationInFrames: 8 });
  const step = frame < HOLD_START ? -1 : frame < HOLD_START + 26 ? 0 : frame < HOLD_END ? 1 : 2;
  const steps = ["Hold a key.", "Speak.", "Release."];
  const cursorOn = Math.floor(frame / 16) % 2 === 0;

  return (
    <Scene duration={duration}>
      <div style={{ display: "flex", flexDirection: "column", alignItems: "center", gap: 44 }}>
        <div style={{ display: "flex", gap: 36 }}>
          {steps.map((label, i) => (
            <span
              key={label}
              style={{
                ...headline,
                fontSize: 76,
                color: i <= step ? C.text : C.track,
              }}
            >
              {label}
            </span>
          ))}
        </div>

        {/* A plain document window */}
        <Rise at={4} style={{ position: "relative" }}>
          <div
            style={{
              width: 1240,
              height: 520,
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
            <div style={{ padding: "44px 56px", fontSize: 44, lineHeight: 1.4 }}>
              {frame >= TEXT_AT && (
                <span>Loudr types what I say, right where my cursor is. </span>
              )}
              <span
                style={{
                  display: "inline-block",
                  width: 4,
                  height: 50,
                  verticalAlign: "-8px",
                  background: C.ink,
                  opacity: cursorOn ? 1 : 0,
                }}
              />
            </div>
          </div>

          {/* The pill, bottom centre of the window */}
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
              fontSize: 28,
            }}
          >
            <span style={{ fontSize: 44 }}>⌥</span>
            <span>right option</span>
          </div>
        </Rise>

        <Rise at={TEXT_AT + 14}>
          <p style={sub}>Text appears in under a second, in any app.</p>
        </Rise>
      </div>
    </Scene>
  );
};

/* 5. Why it is different. */
export const Features: React.FC<{ duration: number }> = ({ duration }) => {
  const cards = [
    ["Free forever", "No account, no word limits, no subscription."],
    ["Fully offline", "Your voice is transcribed on your computer and never leaves it."],
    ["Open source", "MIT licensed. Anyone can add the feature they want."],
  ];
  return (
    <Scene duration={duration}>
      <div style={{ display: "flex", flexDirection: "column", alignItems: "center", gap: 64 }}>
        <Rise>
          <h1 style={headline}>Yours to use. Yours to change.</h1>
        </Rise>
        <div style={{ display: "flex", gap: 28 }}>
          {cards.map(([title, body], i) => (
            <Rise key={title} at={14 + i * 12}>
              <div
                style={{
                  width: 520,
                  height: 300,
                  background: C.panel,
                  borderRadius: 36,
                  padding: 44,
                  display: "flex",
                  flexDirection: "column",
                  gap: 18,
                }}
              >
                <div
                  style={{
                    width: 44,
                    height: 44,
                    borderRadius: 22,
                    background: C.ink,
                    color: C.onInk,
                    display: "grid",
                    placeItems: "center",
                    fontSize: 26,
                    fontWeight: 700,
                  }}
                >
                  ✓
                </div>
                <div style={{ fontSize: 46, fontWeight: 600, letterSpacing: "-0.02em" }}>{title}</div>
                <div style={{ fontSize: 30, color: C.muted, lineHeight: 1.35 }}>{body}</div>
              </div>
            </Rise>
          ))}
        </div>
      </div>
    </Scene>
  );
};

/* 6. Call to action. */
export const Outro: React.FC<{ duration: number }> = () => (
  // No fade-out: the video ends on this frame.
  <Scene duration={100000}>
    <div style={{ display: "flex", flexDirection: "column", alignItems: "center", gap: 30 }}>
      <Rise>
        <Img src={staticFile("icon.png")} style={{ width: 220, height: 220 }} />
      </Rise>
      <Rise at={8}>
        <h1 style={headline}>Loudr</h1>
      </Rise>
      <Rise at={16}>
        <p style={sub}>Free, offline voice typing for macOS and Windows.</p>
      </Rise>
      <Rise at={28} style={{ marginTop: 20 }}>
        <div
          style={{
            fontSize: 40,
            fontWeight: 600,
            padding: "22px 48px",
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
