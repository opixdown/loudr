// Narration made by Loudr itself: each line is spoken by the app's own
// text-to-speech engine (through its `speaks_to_wav` test), so the voice in
// the video is the voice users get.
//
//   LOUDR_VOICE_DIR=<folder with the two voice files> node scripts/voiceover-app.mjs
//   node scripts/voiceover-app.mjs --measure   # keep the .wav files, only re-measure
//
// The voice files are the ones the app downloads, e.g. on macOS
// ~/Library/Application Support/app.loudr.desktop/voices
import { execFileSync } from "node:child_process";
import { mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";

const VOICE = process.env.VOICE ?? "am_puck";
const SPEED = process.env.SPEED ?? "1.18";
const measureOnly = process.argv.includes("--measure");
const LINES = JSON.parse(readFileSync("voiceover-speak-lines.json", "utf8"));

mkdirSync("public/vo-speak", { recursive: true });
const durations = {};
for (const line of LINES) {
  const wav = resolve(`public/vo-speak/${line.id}.wav`);
  if (!measureOnly) {
    execFileSync(
      "cargo",
      ["test", "--release", "speaks_to_wav", "--", "--ignored"],
      {
        cwd: "../src-tauri",
        stdio: "ignore",
        env: { ...process.env, LOUDR_OUT: wav, LOUDR_TEXT: line.say, LOUDR_VOICE: VOICE, LOUDR_SPEED: SPEED },
      },
    );
    rmSync(wav.replace(/\.wav$/, ".mp3"), { force: true }); // the test also writes an MP3
  }
  const info = execFileSync("afinfo", [wav]).toString();
  durations[line.id] = Number(info.match(/estimated duration: ([\d.]+)/)[1]);
}
writeFileSync("src/voiceover-speak.json", JSON.stringify(durations, null, 2) + "\n");
console.log(durations);
