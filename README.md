<p align="center">
  <img src="assets/icon.png" width="120" alt="Loudr icon">
</p>

<h1 align="center">Loudr</h1>

<p align="center">
  <b>Hold a key. Speak. Release. Your words are typed into any app.</b><br>
  Free, open-source, fully offline voice typing for macOS and Windows.<br>
  It also reads text aloud and turns a script into an audio file.<br>
  An alternative to Wispr Flow that never sends your voice anywhere.
</p>

<p align="center">
  <a href="https://loudr-sigma.vercel.app"><b>Website</b></a> ·
  <a href="https://github.com/opixdown/loudr/releases/latest"><b>Download</b></a> ·
  <a href="#build-from-source">Build from source</a> ·
  <a href="#contributing">Contribute</a>
</p>

<p align="center">
  <img src="https://img.shields.io/github/license/opixdown/loudr" alt="MIT license">
  <img src="https://img.shields.io/badge/platform-macOS%20%7C%20Windows-lightgrey" alt="macOS and Windows">
  <img src="https://img.shields.io/github/stars/opixdown/loudr?style=flat" alt="GitHub stars">
</p>

<p align="center">
  <img src="docs/demo.gif" width="720" alt="Loudr promo: hold a key, speak, and the text appears">
</p>

## Why Loudr

- **Offline.** Speech is transcribed on your computer with
  [whisper.cpp](https://github.com/ggerganov/whisper.cpp). The only network
  requests Loudr ever makes are the one-time model downloads.
- **Speaks too.** Select text in any app and tap a key to hear it, or paste
  a script and save it as an audio file. The voice
  ([Kokoro](https://huggingface.co/hexgrad/Kokoro-82M)) also runs on your
  computer.
- **Free, with no limits.** No account, no subscription, no word cap, no
  telemetry.
- **Fast.** The model stays loaded in memory (Metal on Apple Silicon). On an
  M1-class Mac a 5 second clip is typed out in about 0.4 seconds.
- **Small.** The installer is about 22 MB on macOS and 32 MB on Windows, not counting the models.
- **Works everywhere you type.** Loudr pastes into whatever app has focus:
  editors, browsers, chat apps, terminals.
- **Yours to change.** MIT licensed. Add the feature you want and send a pull
  request.

## Install

Download the latest installer from the Releases page:

| System                | File                       |
| --------------------- | -------------------------- |
| macOS (Apple Silicon) | `Loudr_x.y.z_aarch64.dmg`  |
| Windows 10/11 (x64)   | `Loudr_x.y.z_x64_en-US.msi` |

The builds are not signed with a paid developer certificate yet:

- **macOS:** the first time, right-click `Loudr.app` and choose **Open**.
- **Windows:** in the SmartScreen dialog choose **More info**, then **Run anyway**.

On first launch Loudr asks for:

1. **Microphone** access, to hear you while the hotkey is held.
2. **Accessibility** access (macOS only), to notice the hotkey and to paste
   the text into other apps.
3. A **speech model** download (Base, 142 MB, by default).

Text to speech is optional and needs one more download (the voice, about
205 MB), offered on the **Text to speech** tab.

## Use

Hold **Right Option** (macOS) or **Right Alt** (Windows), speak, and release.
A small pill at the bottom of the screen shows a waveform while recording and
a spinner while transcribing.

The menu bar / tray icon has **Settings**, **Text to Speech**, a **Model**
picker and **Quit**.

### Read aloud and text to speech

Open the **Text to speech** tab and download the voice once. Then:

- **Read aloud:** select text in any app and tap **Right Command** (macOS)
  or **Right Ctrl** (Windows). Tap again to stop.
- **Script to audio:** type or paste a script into the box. **Play** speaks
  it; **Save as audio** asks where to put the file and writes an MP3 (or a
  WAV, if you name the file `.wav`).
- **Save what you just heard:** after reading a selection aloud, the tab
  shows it under **Last read aloud** with a **Save audio** button.

There are nine English voices (American and British) and a speed slider.

| Setting         | Choices                                                         |
| --------------- | --------------------------------------------------------------- |
| Hold to talk    | Any modifier key (left or right) or function key; press Change and hit the key |
| Read aloud      | Same choice of keys; a modifier key only triggers when tapped on its own |
| Voice, Speed    | Nine English voices; 0.7× to 1.5×                               |
| Model           | Tiny (75 MB), Base (142 MB), Small (466 MB), Medium (1.5 GB)     |
| Language        | English, auto-detect, and other common languages                |
| Microphone      | System default or a specific device                             |
| Appearance      | Match system, light or dark                                     |
| Launch at login | On or off                                                       |

English uses Whisper's dedicated `.en` models. Any other language uses the
multilingual model of the same size, which is a separate download.

### Good to know

- The text is inserted by pasting. Your previous clipboard **text** is put
  back afterwards, but an image or file on the clipboard is not restored.
- Read aloud copies the selection to get its text, then puts your
  clipboard text back. Speech is English only for now.
- Words that are not in the pronunciation dictionary (unusual names, some
  technical terms) are sounded out by simple rules and can come out wrong.
- The hold key is a single key. Ordinary typing keys can't be used, because
  the hold key stops reaching other apps.
- On Windows the hold key is swallowed while Loudr runs, so Right Alt no
  longer works as AltGr. Pick another hold key if you need AltGr.
- Models are stored in the app data directory
  (`~/Library/Application Support/app.loudr.desktop` on macOS,
  `%APPDATA%\app.loudr.desktop` on Windows), in `models` and `voices`.
- The voice is downloaded from the
  [kokoro-onnx](https://github.com/thewh1teagle/kokoro-onnx) release files.
  Kokoro is Apache 2.0 licensed. MP3 files are written with
  [LAME](https://lame.sourceforge.io) (LGPL), which is compiled into the app.

## Measuring speed

Every dictation logs how long each stage took:

```
timing: audio=5.8s capture_stop=7ms resample=40ms inference=271ms paste=92ms total=420ms chars=23
```

`total` is the time from releasing the key to the paste being sent. The log
is at `~/Library/Logs/app.loudr.desktop/Loudr.log` on macOS and
`%LOCALAPPDATA%\app.loudr.desktop\logs\Loudr.log` on Windows. The most recent
clip is also kept as `last-recording.wav` in the app data directory.

## Build from source

You need [Rust](https://rustup.rs), [Node.js](https://nodejs.org) 20 or newer
and [CMake](https://cmake.org). On macOS also install the Xcode Command Line
Tools. On Windows also install the Visual Studio C++ Build Tools and LLVM
(for `libclang`).

```sh
git clone https://github.com/opixdown/loudr
cd loudr
npm install

npm run tauri dev      # run in development
npm run tauri build    # build the installer into src-tauri/target/release/bundle
```

On macOS, permissions granted to a development build belong to the terminal
you launched it from. To keep permissions across rebuilds of the real app,
sign it with a stable identity:

```sh
APPLE_SIGNING_IDENTITY="Apple Development: you@example.com (TEAMID)" npm run tauri build
```

Pushing a `v*` tag runs `.github/workflows/release.yml`, which builds the
`.dmg` and `.msi` and attaches them to a draft GitHub release.

### Test transcription and speech from the command line

```sh
cd src-tauri
# Transcribe a WAV file
LOUDR_MODEL=/path/to/ggml-base.en.bin LOUDR_WAV=/path/to/clip.wav \
  cargo test --release transcribes_wav_file -- --ignored --nocapture
# Speak a sentence into a WAV file (folder holding the two voice files)
LOUDR_VOICE_DIR=/path/to/voices LOUDR_OUT=out.wav LOUDR_TEXT="Hello there." \
  cargo test --release speaks_to_wav -- --ignored --nocapture
```

## How it works

```
hotkey down -> open microphone (cpal) -> buffer audio, send levels to the pill
hotkey up   -> close microphone -> resample to 16 kHz mono
            -> Whisper (model already in memory) -> clipboard + paste -> restore clipboard
```

| Path                          | What it does                                   |
| ----------------------------- | ---------------------------------------------- |
| `src-tauri/src/hotkey/`       | Native hold-key listener (event tap / keyboard hook) |
| `src-tauri/src/audio.rs`      | Microphone capture, resampling, WAV dump       |
| `src-tauri/src/transcribe.rs` | Whisper model loading and inference            |
| `src-tauri/src/paste.rs`      | Clipboard save, paste shortcut, restore        |
| `src-tauri/src/pipeline.rs`   | The worker thread tying those together, with timing |
| `src-tauri/src/tts/`          | Text to speech: phonemes, Kokoro inference, playback, MP3 / WAV export |
| `src-tauri/src/pill.rs`       | The always-on-top, click-through pill window   |
| `src-tauri/src/models.rs`     | Model files and downloads                      |
| `src-tauri/src/settings.rs`   | Settings file                                  |
| `src-tauri/src/tray.rs`       | Tray icon and menu                             |
| `src/windows/`                | React UI: pill, first-run flow, settings       |
| `site/`                       | The website (static HTML, hosted on Vercel)    |
| `video/`                      | The promo videos (Remotion)                    |

## Not in v1

No cloud APIs, no AI rewriting, no accounts, no auto-update, no Linux, no
Intel Mac build, no CUDA.

## Contributing

Loudr is meant to be built on. Issues and pull requests are welcome. Ideas
that would make good first contributions:

- Linux support
- An Intel Mac build
- A custom vocabulary for names and technical terms
- Restoring images and files on the clipboard after pasting
- GPU acceleration on Windows

For anything large, open an issue first so we can agree on the approach.

## License

[MIT](LICENSE)
