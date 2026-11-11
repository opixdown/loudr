# Loudr

Free, open-source, fully offline voice typing for macOS and Windows.
An alternative to Wispr Flow that never sends your voice anywhere.

Hold a key, speak, release. Your words are typed into whatever app is focused.

![Demo](docs/demo.gif)

<!-- TODO: record a short demo GIF and save it as docs/demo.gif -->

- **Offline.** Speech is transcribed on your computer with
  [whisper.cpp](https://github.com/ggerganov/whisper.cpp). The only network
  request Loudr ever makes is the one-time model download.
- **No account, no telemetry, no cloud.**
- **Fast.** The model stays loaded in memory (Metal on Apple Silicon), so a
  5 second clip is typed out in well under a second on an M1.
- **Small.** The installer is a few megabytes, not counting the model.

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

## Use

Hold **Right Option** (macOS) or **Right Alt** (Windows), speak, and release.
A small pill at the bottom of the screen shows a waveform while recording and
a spinner while transcribing.

The menu bar / tray icon has **Settings**, a **Model** picker and **Quit**.

| Setting         | Choices                                                         |
| --------------- | --------------------------------------------------------------- |
| Hold to talk    | Any modifier key (left or right) or function key; press Change and hit the key |
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
- The hold key is a single key. Ordinary typing keys can't be used, because
  the hold key stops reaching other apps.
- On Windows the hold key is swallowed while Loudr runs, so Right Alt no
  longer works as AltGr. Pick another hold key if you need AltGr.
- Models are stored in the app data directory
  (`~/Library/Application Support/app.loudr.desktop/models` on macOS,
  `%APPDATA%\app.loudr.desktop\models` on Windows).

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

### Test transcription without a microphone

```sh
cd src-tauri
LOUDR_MODEL=/path/to/ggml-base.en.bin LOUDR_WAV=/path/to/clip.wav \
  cargo test -- --ignored --nocapture
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
| `src-tauri/src/pill.rs`       | The always-on-top, click-through pill window   |
| `src-tauri/src/models.rs`     | Model files and downloads                      |
| `src-tauri/src/settings.rs`   | Settings file                                  |
| `src-tauri/src/tray.rs`       | Tray icon and menu                             |
| `src/windows/`                | React UI: pill, first-run flow, settings       |

## Not in v1

No cloud APIs, no AI rewriting, no accounts, no auto-update, no Linux, no
Intel Mac build, no CUDA.

## License

[MIT](LICENSE)
