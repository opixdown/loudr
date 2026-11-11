// Small building blocks shared by the scenes.
import React from "react";
import { AbsoluteFill, interpolate, spring, useCurrentFrame, useVideoConfig } from "remotion";
import { C, FONT } from "./theme";

/** Full-frame scene background that fades the whole scene out at its end. */
export const Scene: React.FC<{ duration: number; children: React.ReactNode }> = ({
  duration,
  children,
}) => {
  const frame = useCurrentFrame();
  const opacity = interpolate(frame, [duration - 10, duration], [1, 0], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });
  return (
    <AbsoluteFill
      style={{
        background: C.bg,
        color: C.text,
        fontFamily: FONT,
        alignItems: "center",
        justifyContent: "center",
        opacity,
      }}
    >
      {children}
    </AbsoluteFill>
  );
};

/** Fades and slides its children in, starting at frame `at`. */
export const Rise: React.FC<{
  at?: number;
  distance?: number;
  style?: React.CSSProperties;
  children: React.ReactNode;
}> = ({ at = 0, distance = 28, style, children }) => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const t = spring({ frame: frame - at, fps, config: { damping: 200 } });
  return (
    <div style={{ opacity: t, transform: `translateY(${(1 - t) * distance}px)`, ...style }}>
      {children}
    </div>
  );
};

/** The app's waveform: rounded bars whose heights follow a fake voice. */
export const Waveform: React.FC<{
  bars?: number;
  barWidth?: number;
  gap?: number;
  height?: number;
  color?: string;
  /** 0 = silent, 1 = speaking. */
  energy?: number;
}> = ({ bars = 28, barWidth = 6, gap = 4, height = 48, color = C.ink, energy = 1 }) => {
  const frame = useCurrentFrame();
  return (
    <div style={{ display: "flex", alignItems: "center", gap, height }}>
      {Array.from({ length: bars }, (_, i) => {
        // The waveform scrolls: bar i shows the level from (bars - i) frames ago.
        const t = frame - (bars - i);
        const voice =
          0.5 +
          0.3 * Math.sin(t * 0.55) +
          0.2 * Math.sin(t * 0.21 + 1.3) +
          0.15 * Math.sin(t * 1.3 + 0.4);
        // Short syllable-like dips, so it reads as speech and not a tone.
        const gate = Math.sin(t * 0.16) > -0.8 ? 1 : 0.25;
        const level = Math.max(0, Math.min(1, voice * gate)) * energy;
        return (
          <div
            key={i}
            style={{
              width: barWidth,
              height: Math.max(barWidth, level * height),
              borderRadius: barWidth / 2,
              background: color,
            }}
          />
        );
      })}
    </div>
  );
};

/** The floating pill, as in the app. */
export const Pill: React.FC<{ children: React.ReactNode; scale?: number }> = ({
  children,
  scale = 1,
}) => (
  <div
    style={{
      display: "flex",
      alignItems: "center",
      justifyContent: "center",
      minHeight: 80 * scale,
      padding: `${16 * scale}px ${32 * scale}px`,
      borderRadius: 999,
      background: C.control,
      border: `2px solid rgba(26,25,23,0.22)`,
    }}
  >
    {children}
  </div>
);

export const Spinner: React.FC<{ size?: number }> = ({ size = 36 }) => {
  const frame = useCurrentFrame();
  return (
    <div
      style={{
        width: size,
        height: size,
        borderRadius: "50%",
        border: `4px solid ${C.track}`,
        borderTopColor: C.ink,
        transform: `rotate(${frame * 17}deg)`,
      }}
    />
  );
};
