"""Assemble the film's soundtrack: music, narration and sound effects, placed
on the cues the recorder wrote (frames/cues.json).

    python3 audio.py        (needs numpy)

Writes audio/mix.wav, 44.1 kHz stereo.
"""
import json
import wave
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
ASSETS = HERE.parent / "public"  # music.wav and sfx/ from the earlier videos
RATE = 44100

MUSIC_UNDER = 0.15  # under the voice
MUSIC_OUTRO = 0.42  # after the last line
VOICE_DELAY = 0.28  # seconds after a beat starts before the voice comes in

# Narration lines (audio/vo/<name>.wav), keyed by the cue that starts them.
VOICE = ["title", "connect", "key", "play", "save", "trust", "outro"]
# Sound effects: cue name -> (file in public/sfx, volume)
EFFECTS = {
    "click": ("key", 0.42),
    "done": ("ding", 0.3),
    "whoosh": ("whoosh", 0.3),
    "pop": ("pop", 0.3),
    # Each new shot arrives with a soft whoosh
    "connect": ("whoosh", 0.2),
    "trust": ("whoosh", 0.2),
    "outro": ("ding", 0.26),
}


def load(path: Path) -> np.ndarray:
    """A WAV file as float stereo at RATE."""
    with wave.open(str(path)) as w:
        rate, channels = w.getframerate(), w.getnchannels()
        data = np.frombuffer(w.readframes(w.getnframes()), dtype=np.int16).astype(np.float32) / 32768
    data = data.reshape(-1, channels)
    if rate != RATE:
        at = np.arange(int(len(data) * RATE / rate)) * rate / RATE
        data = np.stack([np.interp(at, np.arange(len(data)), data[:, c]) for c in range(channels)], axis=1)
    return np.repeat(data, 2, axis=1) if channels == 1 else data


record = json.loads((HERE / "frames/cues.json").read_text())
fps, cues = record["fps"], record["cues"]
seconds = record["frames"] / fps
mix = np.zeros((int(seconds * RATE), 2), dtype=np.float32)


def add(at: float, sound: np.ndarray, volume: float = 1.0) -> None:
    start = int(at * RATE)
    part = sound[: max(0, len(mix) - start)]
    mix[start : start + len(part)] += part * volume


time_of = lambda name: [c["frame"] / fps for c in cues if c["name"] == name]

# Narration, and when the last line ends (the music comes up after it)
voice_end = 0.0
for name in VOICE:
    line = load(HERE / f"audio/vo/{name}.wav")
    start = time_of(name)[0] + (0.5 if name == "title" else VOICE_DELAY)
    add(start, line, 1.0)
    voice_end = start + len(line) / RATE

for name, (file, volume) in EFFECTS.items():
    sound = load(ASSETS / f"sfx/{file}.wav")
    for at in time_of(name):
        add(at, sound, volume)

# Typing: light key taps, a little uneven like a person, for as long as the
# script takes to type.
tap = load(ASSETS / "sfx/key.wav")
rng = np.random.default_rng(7)
for start, end in zip(time_of("typing"), time_of("typed")):
    at = start
    while at < end:
        add(at, tap, 0.1 + 0.07 * rng.random())
        at += 0.075 + 0.05 * rng.random()

# Music: quiet under the voice, up for the sign-off, faded at both ends
music = load(ASSETS / "music.wav")
music = np.tile(music, (int(np.ceil(len(mix) / len(music))), 1))[: len(mix)]
t = np.arange(len(mix)) / RATE
lift = np.clip((t - voice_end) / 0.5, 0, 1)
level = (MUSIC_UNDER + (MUSIC_OUTRO - MUSIC_UNDER) * lift) * np.clip(t / 0.6, 0, 1) * np.clip((seconds - t) / 1.4, 0, 1)
mix += music * level[:, None]

peak = float(np.abs(mix).max())
if peak > 0.97:
    mix *= 0.97 / peak
out = HERE / "audio/mix.wav"
with wave.open(str(out), "w") as w:
    w.setnchannels(2)
    w.setsampwidth(2)
    w.setframerate(RATE)
    w.writeframes((mix * 32767).astype(np.int16).tobytes())
print(f"wrote {out.name}: {seconds:.1f}s, peak {peak:.2f}")
