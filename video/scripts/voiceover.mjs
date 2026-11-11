// Fallback voiceover using the macOS `say` command (robotic; prefer
// scripts/voiceover.py, which uses a natural-sounding local voice model).
// Also records how long each line is, so every scene matches its narration.
//
//   node scripts/voiceover.mjs            # generate with the default voice
//   VOICE=Samantha node scripts/voiceover.mjs
//   node scripts/voiceover.mjs --measure  # keep the .wav files, only re-measure
//
// To use your own voice: record one file per line, save them over
// public/vo/<id>.wav, then run with --measure and render again.
import { execFileSync } from "node:child_process";
import { mkdirSync, readFileSync, writeFileSync, rmSync } from "node:fs";

const VOICE = process.env.VOICE ?? "Daniel";
const RATE = process.env.RATE ?? "190"; // words per minute
const measureOnly = process.argv.includes("--measure");

// `say` is what gets spoken (spelled so the voice pronounces the names right).
const LINES = JSON.parse(readFileSync("voiceover-lines.json", "utf8"));

mkdirSync("public/vo", { recursive: true });
const durations = {};
for (const line of LINES) {
  const wav = `public/vo/${line.id}.wav`;
  if (!measureOnly) {
    const aiff = `public/vo/${line.id}.aiff`;
    execFileSync("say", ["-v", VOICE, "-r", RATE, "-o", aiff, line.say]);
    execFileSync("afconvert", ["-f", "WAVE", "-d", "LEI16@48000", aiff, wav]);
    rmSync(aiff);
  }
  const info = execFileSync("afinfo", [wav]).toString();
  durations[line.id] = Number(info.match(/estimated duration: ([\d.]+)/)[1]);
}
writeFileSync("src/voiceover.json", JSON.stringify(durations, null, 2) + "\n");
console.log(durations);
