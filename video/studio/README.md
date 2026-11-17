# Studio voices film

A 34 second launch film for Loudr 0.4, 1920x1080 with sound.

The picture is one web page (`src/Film.tsx`) built with the
[Stageshow](kit/LICENSE) kit: a believable copy of Loudr's settings window,
a cursor that presses and types, and full-frame title cards. It is recorded
frame by frame on a virtual clock, so the result is the same on any computer.

```
npm install
node record.mjs        # frames/00000.jpg ... and frames/cues.json
python3 audio.py       # audio/mix.wav (needs numpy)
ffmpeg -framerate 30 -i frames/%05d.jpg -i audio/mix.wav \
  -vf "scale=out_color_matrix=bt709:out_range=tv,format=yuv420p" \
  -c:v libx264 -preset slow -crf 16 -c:a aac -b:a 192k -shortest \
  -movflags +faststart out/loudr-studio.mp4
```

- `record.mjs` needs a Chromium "headless shell"; set `CHROME` to its path.
- The script marks each beat with a cue. `audio.py` places the narration and
  sound effects on those cues, so changing the timing in `Film.tsx` never
  puts the sound out of step.
- Narration (`audio/vo/`) is spoken by Loudr's own offline voice. Music and
  sound effects are the ones in `../public`, made by `../scripts`.
- `npm run dev` plays the film live at http://localhost:1430 for editing.
