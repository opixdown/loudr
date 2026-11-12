import React from "react";
import { Audio, Composition, Sequence, Series, staticFile } from "remotion";
import { Demo, Free, Hook, Offline, OpenSource, Outro, Problem, Reveal } from "./scenes";
import { Intro, READ_TAP, ReadAloud, SCRIPT_SAVE, Script, Turn } from "./speakScenes";
import { FPS } from "./theme";
import promoVoice from "./voiceover.json";
import speakVoice from "./voiceover-speak.json";

/** A sound effect inside a scene: file in public/sfx, start as a fraction of the scene. */
type Sfx = { file: string; at: number; volume?: number };
type SceneDef = {
  /** Voiceover line; its length (plus `pad` seconds of breath) sets the scene's length. */
  id: string;
  scene: React.FC<{ duration: number }>;
  pad: number;
  sfx?: Sfx[];
};
type Video = {
  /** Folder in public/ holding <id>.wav, and the measured length of each. */
  voiceDir: string;
  lengths: Record<string, number>;
  scenes: SceneDef[];
  /** Play a soft whoosh at every scene change. */
  whoosh?: boolean;
};

/** Frames of silence before the voice starts in each scene. */
const LEAD_IN = 4;
// Background music (public/music.wav, made by scripts/music.py): quiet under
// the voice, louder for the closing shot, faded at both ends.
const MUSIC_UNDER = 0.16;
const MUSIC_OUTRO = 0.42;

const sceneFrames = (video: Video, s: SceneDef) =>
  LEAD_IN + Math.ceil((video.lengths[s.id] + s.pad) * FPS);
const totalFrames = (video: Video) =>
  video.scenes.reduce((sum, s) => sum + sceneFrames(video, s), 0);

const VideoComp: React.FC<{ video: Video }> = ({ video }) => {
  const total = totalFrames(video);
  const last = video.scenes[video.scenes.length - 1];
  // When the last voiceover line ends; the music comes up after it.
  const voiceEnd = total - Math.ceil(last.pad * FPS);
  const musicVolume = (f: number) => {
    const fadeIn = Math.min(1, f / (FPS * 0.5));
    const fadeOut = Math.max(0, Math.min(1, (total - f) / (FPS * 1.2)));
    const lift = Math.min(1, Math.max(0, (f - voiceEnd) / (FPS * 0.4)));
    return (MUSIC_UNDER + (MUSIC_OUTRO - MUSIC_UNDER) * lift) * fadeIn * fadeOut;
  };
  return (
    <>
      <Audio src={staticFile("music.wav")} volume={musicVolume} />
      <Series>
        {video.scenes.map((s, index) => {
          const frames = sceneFrames(video, s);
          return (
            <Series.Sequence key={s.id} durationInFrames={frames}>
              <s.scene duration={frames} />
              <Sequence from={LEAD_IN}>
                <Audio src={staticFile(`${video.voiceDir}/${s.id}.wav`)} />
              </Sequence>
              {video.whoosh && index > 0 && (
                <Audio src={staticFile("sfx/whoosh.wav")} volume={0.22} />
              )}
              {s.sfx?.map((fx) => (
                <Sequence key={fx.file + fx.at} from={Math.round(fx.at * frames)}>
                  <Audio src={staticFile(`sfx/${fx.file}.wav`)} volume={fx.volume ?? 0.5} />
                </Sequence>
              ))}
            </Series.Sequence>
          );
        })}
      </Series>
    </>
  );
};

// The launch video: why Loudr exists.
const PROMO: Video = {
  voiceDir: "vo",
  lengths: promoVoice,
  scenes: [
    { id: "hook", scene: Hook, pad: 0.35 },
    { id: "problem", scene: Problem, pad: 0.35 },
    { id: "reveal", scene: Reveal, pad: 0.55 },
    { id: "demo", scene: Demo, pad: 0.45 },
    { id: "offline", scene: Offline, pad: 0.35 },
    { id: "free", scene: Free, pad: 0.35 },
    { id: "open", scene: OpenSource, pad: 0.4 },
    { id: "outro", scene: Outro, pad: 2.2 }, // hold the last frame
  ],
};

// The text-to-speech video. Its narration is spoken by Loudr itself
// (scripts/voiceover-app.mjs).
const SPEAK: Video = {
  voiceDir: "vo-speak",
  lengths: speakVoice,
  whoosh: true,
  scenes: [
    { id: "intro", scene: Intro, pad: 0.4, sfx: [{ file: "pop", at: 0.02, volume: 0.4 }] },
    { id: "turn", scene: Turn, pad: 0.4, sfx: [{ file: "pop", at: 0.56, volume: 0.35 }] },
    { id: "read", scene: ReadAloud, pad: 1.1, sfx: [{ file: "key", at: READ_TAP, volume: 0.7 }] },
    {
      id: "script",
      scene: Script,
      pad: 0.9,
      sfx: [
        { file: "pop", at: 0.1, volume: 0.3 },
        { file: "key", at: SCRIPT_SAVE, volume: 0.6 },
        { file: "ding", at: SCRIPT_SAVE + 0.04, volume: 0.45 },
      ],
    },
    {
      id: "offline",
      scene: (p) => (
        <Offline {...p} title="Runs on your computer." line="Nothing you select is sent anywhere." />
      ),
      pad: 0.35,
    },
    {
      id: "free",
      scene: (p) => (
        <Free
          {...p}
          lines={[
            ["Nine voices.", 0.02],
            ["No account.", 0.4],
            ["No limits.", 0.7],
          ]}
        />
      ),
      pad: 0.35,
    },
    {
      id: "outro",
      scene: (p) => (
        <Outro {...p} line="Voice typing and text to speech. Free, offline, open source." />
      ),
      pad: 2.4,
      sfx: [{ file: "ding", at: 0.05, volume: 0.3 }],
    },
  ],
};

export const Root: React.FC = () => (
  <>
    <Composition
      id="Promo"
      component={() => <VideoComp video={PROMO} />}
      durationInFrames={totalFrames(PROMO)}
      fps={FPS}
      width={1920}
      height={1080}
    />
    <Composition
      id="Speak"
      component={() => <VideoComp video={SPEAK} />}
      durationInFrames={totalFrames(SPEAK)}
      fps={FPS}
      width={1920}
      height={1080}
    />
  </>
);
