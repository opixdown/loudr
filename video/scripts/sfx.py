"""Synthesize the sound effects (public/sfx/*.wav). Made from scratch with
numpy, like the music, so there is nothing to license.

    .tts/.venv/bin/python scripts/sfx.py
"""
from pathlib import Path

import numpy as np
import soundfile as sf

RATE = 44100
OUT = Path(__file__).resolve().parent.parent / "public/sfx"
OUT.mkdir(parents=True, exist_ok=True)
rng = np.random.default_rng(3)


def t(seconds: float) -> np.ndarray:
    return np.arange(int(seconds * RATE)) / RATE


def lowpass(x: np.ndarray, cutoff: np.ndarray) -> np.ndarray:
    """One-pole low-pass whose cutoff (Hz) can change over time."""
    a = 1 - np.exp(-2 * np.pi * cutoff / RATE)
    y = np.zeros_like(x)
    acc = 0.0
    for i in range(len(x)):
        acc += a[i] * (x[i] - acc)
        y[i] = acc
    return y


def save(name: str, x: np.ndarray, peak: float) -> None:
    x = x * peak / np.abs(x).max()
    sf.write(OUT / f"{name}.wav", x, RATE, subtype="PCM_16")
    print("wrote", name)


# Whoosh: filtered noise that swells and opens up, for scene changes.
x = t(0.45)
noise = rng.standard_normal(len(x))
sweep = lowpass(noise, 300 + 5000 * np.sin(np.pi * x / 0.45) ** 2)
save("whoosh", sweep * np.sin(np.pi * x / 0.45) ** 2, 0.5)

# Key press: a short click with a soft low thump under it.
x = t(0.12)
click = rng.standard_normal(len(x)) * np.exp(-x * 220)
thump = np.sin(2 * np.pi * 150 * x * (1 - 2.5 * x)) * np.exp(-x * 45)
save("key", lowpass(click, np.full(len(x), 4500.0)) * 0.7 + thump, 0.8)

# Pop: a quick rising blip, for things appearing.
x = t(0.16)
pop = np.sin(2 * np.pi * (420 * x + 2400 * x**2)) * np.exp(-x * 30) * np.minimum(1, x / 0.004)
save("pop", pop, 0.6)

# Ding: two soft bell notes a fifth apart, for "saved".
x = t(1.1)
def bell(freq: float, start: float) -> np.ndarray:
    y = np.maximum(0, x - start)
    tone = np.sin(2 * np.pi * freq * y) + 0.35 * np.sin(2 * np.pi * freq * 2.76 * y) * np.exp(-y * 9)
    return tone * np.exp(-y * 5) * (x >= start) * np.minimum(1, y / 0.003)
save("ding", bell(880, 0) + bell(1318.5, 0.11), 0.6)
