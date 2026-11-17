// Record the film frame by frame.
//
//   node record.mjs [--frames N] [--every N]
//
// The page runs on a virtual clock (src/main.ts): time moves only when this
// script steps it, one thirtieth of a second at a time, and a picture is
// taken after each step. Nothing depends on how fast this computer is.
// Writes frames/00000.jpg ... and frames/cues.json (which frame each cue in
// the script landed on), which audio.py uses to place the sound.
import { spawn } from 'node:child_process'
import { mkdirSync, rmSync, writeFileSync } from 'node:fs'
import { chromium } from 'playwright-core'

const arg = (name, fallback) => {
  const i = process.argv.indexOf(`--${name}`)
  return i > 0 ? Number(process.argv[i + 1]) : fallback
}
const MAX_FRAMES = arg('frames', 30 * 60)
/** Keep one frame in N (for a quick look at the timing). */
const EVERY = arg('every', 1)
const URL = 'http://localhost:1430/?record'
const CHROME =
  process.env.CHROME ??
  `${process.env.HOME}/Library/Caches/ms-playwright/chromium_headless_shell-1248/chrome-headless-shell-mac-arm64/chrome-headless-shell`

const server = spawn('npx', ['vite'], { stdio: 'ignore' })
const sleep = (ms) => new Promise((r) => setTimeout(r, ms))
let browser
try {
  for (let i = 0; i < 60; i++) {
    if (await fetch(URL).then((r) => r.ok, () => false)) break
    await sleep(250)
  }
  rmSync('frames', { recursive: true, force: true })
  mkdirSync('frames')

  browser = await chromium.launch({ executablePath: CHROME, args: ['--use-angle=swiftshader', '--enable-unsafe-swiftshader'] })
  // 1280x720 drawn at 1.5x is 1920x1080.
  const page = await browser.newPage({ viewport: { width: 1280, height: 720 }, deviceScaleFactor: 1.5 })
  page.on('console', (m) => ['error', 'warning'].includes(m.type()) && console.log(`[page ${m.type()}]`, m.text()))
  page.on('pageerror', (e) => console.log('[page error]', e.message))
  await page.goto(URL)
  // Polled from here: the page's own timers are frozen until stepped.
  for (let i = 0; !(await page.evaluate(() => window.__filmReady === true)); i++) {
    if (i > 120) throw new Error('The film did not load')
    await sleep(250)
  }

  const cues = []
  let last = null
  let endAt = Infinity
  let saved = 0
  for (let frame = 0; frame < Math.min(MAX_FRAMES, endAt); frame++) {
    const mark = await page.evaluate(async () => {
      await window.__stageshow.clock.step()
      return document.querySelector('[data-mark]')?.getAttribute('data-mark') ?? null
    })
    if (mark && mark !== last) {
      last = mark
      cues.push({ frame, name: mark.split('#')[0] })
      if (mark.startsWith('end')) endAt = frame + 15
    }
    if (frame % EVERY === 0) {
      await page.screenshot({ path: `frames/${String(saved++).padStart(5, '0')}.jpg`, type: 'jpeg', quality: 96 })
    }
    if (frame % 150 === 0) console.log(`frame ${frame}`)
  }
  writeFileSync('frames/cues.json', JSON.stringify({ fps: 30 / EVERY, frames: saved, cues }, null, 2))
  console.log(`wrote ${saved} frames, ${cues.length} cues`)
} finally {
  await browser?.close()
  server.kill()
}
