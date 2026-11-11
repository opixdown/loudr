"""Synthesize the background music bed (public/music.wav).

Written from scratch with numpy, so there is no licence to worry about: a
warm pad, a plucked arpeggio, a soft kick and a sub bass at 100 BPM.

    .tts/.venv/bin/python scripts/music.py [seconds]
"""
import sys
from pathlib import Path

import numpy as np
import soundfile as sf

RATE = 44100
BPM = 100
BEAT = 60 / BPM
BAR = 4 * BEAT
ROOT = Path(__file__).resolve().parent.parent
seconds = float(sys.argv[1]) if len(sys.argv) > 1 else 45.0

# Fmaj7 - Am7 - Dm7 - Cmaj7, as MIDI notes. One chord per bar.
CHORDS = [
    (41, [65, 69, 72, 76]),
    (45, [64, 67, 69, 72]),
    (38, [62, 65, 69, 72]),
    (36, [64, 67, 71, 72]),
]

n_total = int(seconds * RATE)
left = np.zeros(n_total + RATE * 4)
right = np.zeros_like(left)


def hz(midi: float) -> float:
    return 440.0 * 2 ** ((midi - 69) / 12)


def add(start: float, sound: np.ndarray, pan: float = 0.0) -> None:
    i = int(start * RATE)
    if i >= n_total:
        return
    left[i : i + len(sound)] += sound * (1 - pan) / 2 * 2 ** 0.5
    right[i : i + len(sound)] += sound * (1 + pan) / 2 * 2 ** 0.5


def pad(midi: int, length: float) -> np.ndarray:
    t = np.arange(int(length * RATE)) / RATE
    f = hz(midi)
    # Two slightly detuned voices with a few soft harmonics.
    s = sum(
        np.sin(2 * np.pi * f * d * h * t) / h**2
        for d in (0.997, 1.003)
        for h in (1, 2, 3)
    )
    attack, release = 0.6, 0.9
    env = np.minimum(1, t / attack) * np.minimum(1, (length - t) / release)
    return s * env * 0.05


def pluck(midi: int, length: float = 0.9) -> np.ndarray:
    t = np.arange(int(length * RATE)) / RATE
    f = hz(midi)
    s = np.sin(2 * np.pi * f * t) + 0.35 * np.sin(2 * np.pi * 2 * f * t) * np.exp(-t * 9)
    env = np.minimum(1, t / 0.004) * np.exp(-t * 5.5)
    return s * env * 0.11


def kick() -> np.ndarray:
    t = np.arange(int(0.32 * RATE)) / RATE
    phase = 2 * np.pi * (48 * t + (110 - 48) / 28 * (1 - np.exp(-28 * t)))
    return np.sin(phase) * np.exp(-t * 13) * 0.42


def bass(midi: int, length: float) -> np.ndarray:
    t = np.arange(int(length * RATE)) / RATE
    env = np.minimum(1, t / 0.02) * np.minimum(1, (length - t) / 0.08)
    return np.sin(2 * np.pi * hz(midi) * t) * env * 0.2


def hat() -> np.ndarray:
    t = np.arange(int(0.05 * RATE)) / RATE
    noise = np.random.default_rng(7).standard_normal(len(t))
    noise = np.diff(noise, prepend=0)  # crude high-pass
    return noise * np.exp(-t * 90) * 0.035


bars = int(seconds / BAR) + 1
for bar in range(bars):
    root, notes = CHORDS[bar % len(CHORDS)]
    t0 = bar * BAR
    for i, note in enumerate(notes):
        add(t0, pad(note - 12, BAR + 0.8), pan=(i - 1.5) * 0.3)

    # The first two bars are pad only; the rest come in as the hook lands.
    if bar >= 1:
        pattern = [0, 2, 1, 3, 2, 1, 3, 2]  # eighth notes through the chord
        for step, idx in enumerate(pattern):
            add(t0 + step * BEAT / 2, pluck(notes[idx]), pan=0.35 if step % 2 else -0.35)
            # A quiet echo a dotted eighth later.
            add(t0 + step * BEAT / 2 + BEAT * 0.75, pluck(notes[idx]) * 0.3, pan=-0.35 if step % 2 else 0.35)
    if bar >= 2:
        for beat in (0, 2):
            add(t0 + beat * BEAT, kick())
        add(t0, bass(root, BEAT * 1.5))
        add(t0 + BEAT * 2.5, bass(root, BEAT * 1.2))
        for step in range(8):
            if step % 2:
                add(t0 + step * BEAT / 2, hat(), pan=0.2)

mix = np.stack([left[:n_total], right[:n_total]], axis=1)
mix = np.tanh(mix * 1.2)  # gentle limiting
mix *= 0.8 / np.abs(mix).max()
sf.write(ROOT / "public/music.wav", mix, RATE, subtype="PCM_16")
print(f"wrote public/music.wav ({seconds:.1f}s)")
