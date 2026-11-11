import React from "react";
import { Composition, Series } from "remotion";
import { Demo, Features, Hook, Outro, Problem, Reveal } from "./scenes";
import { FPS } from "./theme";

// Scene order and length in frames (30 per second).
const SCENES: [React.FC<{ duration: number }>, number][] = [
  [Hook, 100],
  [Problem, 150],
  [Reveal, 100],
  [Demo, 220],
  [Features, 160],
  [Outro, 130],
];

const Promo: React.FC = () => (
  <Series>
    {SCENES.map(([SceneComponent, duration], i) => (
      <Series.Sequence key={i} durationInFrames={duration}>
        <SceneComponent duration={duration} />
      </Series.Sequence>
    ))}
  </Series>
);

export const Root: React.FC = () => (
  <Composition
    id="Promo"
    component={Promo}
    durationInFrames={SCENES.reduce((sum, [, d]) => sum + d, 0)}
    fps={FPS}
    width={1920}
    height={1080}
  />
);
