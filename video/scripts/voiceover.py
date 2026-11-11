"""Generate the voiceover with Kokoro, an open-source neural voice that runs
locally (no account, no cloud), then measure the lines for scene timing.

One-time setup, from the video/ folder:
    mkdir -p .tts && cd .tts
    uv venv --python 3.12 && uv pip install --python .venv/bin/python kokoro-onnx soundfile
    curl -LO https://github.com/thewh1teagle/kokoro-onnx/releases/download/model-files-v1.0/kokoro-v1.0.onnx
    curl -LO https://github.com/thewh1teagle/kokoro-onnx/releases/download/model-files-v1.0/voices-v1.0.bin

Then:
    .tts/.venv/bin/python scripts/voiceover.py                 # default voice
    .tts/.venv/bin/python scripts/voiceover.py am_fenrir 1.1   # voice, speed
    .tts/.venv/bin/python scripts/voiceover.py --samples       # one line in several voices
"""
import json
import subprocess
import sys
from pathlib import Path

import numpy as np
import soundfile as sf
from kokoro_onnx import Kokoro

ROOT = Path(__file__).resolve().parent.parent
LINES = json.loads((ROOT / "voiceover-lines.json").read_text())
kokoro = Kokoro(str(ROOT / ".tts/kokoro-v1.0.onnx"), str(ROOT / ".tts/voices-v1.0.bin"))


def speak(text: str, voice: str, speed: float, path: Path) -> None:
    lang = "en-gb" if voice.startswith("b") else "en-us"
    samples, rate = kokoro.create(text, voice=voice, speed=speed, lang=lang)
    # Leave a little headroom and trim nothing: the scenes add their own pauses.
    samples = samples / max(1.0, float(np.abs(samples).max()) / 0.95)
    path.parent.mkdir(parents=True, exist_ok=True)
    sf.write(path, samples, rate, subtype="PCM_16")


if "--samples" in sys.argv:
    # The same two lines in several voices, to choose between by ear.
    text = f'{LINES[0]["say"]} {LINES[2]["say"]}'
    for voice in ["am_michael", "am_fenrir", "am_puck", "am_onyx", "af_heart", "af_bella", "bm_george", "bf_emma"]:
        speak(text, voice, 1.05, ROOT / f"out/voice-samples/{voice}.wav")
        print("wrote", voice)
else:
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    voice = args[0] if args else "am_puck"
    speed = float(args[1]) if len(args) > 1 else 1.05
    for line in LINES:
        speak(line["say"], voice, speed, ROOT / f'public/vo/{line["id"]}.wav')
    # Record each line's length so the scenes match the narration.
    subprocess.run(["node", "scripts/voiceover.mjs", "--measure"], cwd=ROOT, check=True)
