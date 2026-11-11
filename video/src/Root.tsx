import React from "react";
import { Audio, Composition, Sequence, Series, staticFile } from "remotion";
import { Demo, Free, Hook, Offline, OpenSource, Outro, Problem, Reveal } from "./scenes";
import { FPS } from "./theme";
import voiceover from "./voiceover.json";

// Each scene lasts as long as its voiceover line (public/vo/<id>.wav, measured
// into voiceover.json by scripts/voiceover.mjs) plus a short breath after it.
const SCENES: { id: keyof typeof voiceover; scene: React.FC<{ duration: number }>; pad: number }[] = [
  { id: "hook", scene: Hook, pad: 0.35 },
  { id: "problem", scene: Problem, pad: 0.35 },
  { id: "reveal", scene: Reveal, pad: 0.55 },
  { id: "demo", scene: Demo, pad: 0.45 },
  { id: "offline", scene: Offline, pad: 0.35 },
  { id: "free", scene: Free, pad: 0.35 },
  { id: "open", scene: OpenSource, pad: 0.4 },
  { id: "outro", scene: Outro, pad: 2.2 }, // hold the last frame
];
/** Frames of silence before the voice starts in each scene. */
const LEAD_IN = 4;

const frames = (s: (typeof SCENES)[number]) =>
  LEAD_IN + Math.ceil((voiceover[s.id] + s.pad) * FPS);

const TOTAL = SCENES.reduce((sum, s) => sum + frames(s), 0);
/** When the last voiceover line ends; the music comes up after it. */
const VOICE_END = TOTAL - Math.ceil(SCENES[SCENES.length - 1].pad * FPS);

// Background music (public/music.wav, made by scripts/music.py): quiet under
// the voice, louder for the closing shot, faded at both ends.
const MUSIC_UNDER = 0.16;
const MUSIC_OUTRO = 0.42;
const musicVolume = (f: number) => {
  const fadeIn = Math.min(1, f / (FPS * 0.5));
  const fadeOut = Math.min(1, (TOTAL - f) / (FPS * 1.2));
  const lift = Math.min(1, Math.max(0, (f - VOICE_END) / (FPS * 0.4)));
  return (MUSIC_UNDER + (MUSIC_OUTRO - MUSIC_UNDER) * lift) * fadeIn * Math.max(0, fadeOut);
};

const Promo: React.FC = () => (
  <>
    <Audio src={staticFile("music.wav")} volume={musicVolume} />
    <Series>
      {SCENES.map((s) => (
        <Series.Sequence key={s.id} durationInFrames={frames(s)}>
          <s.scene duration={frames(s)} />
          <Sequence from={LEAD_IN}>
            <Audio src={staticFile(`vo/${s.id}.wav`)} />
          </Sequence>
        </Series.Sequence>
      ))}
    </Series>
  </>
);

export const Root: React.FC = () => (
  <Composition
    id="Promo"
    component={Promo}
    durationInFrames={TOTAL}
    fps={FPS}
    width={1920}
    height={1080}
  />
);
