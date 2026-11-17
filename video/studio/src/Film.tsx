// The Studio voices launch film, as one stage.
//
// Four shots share the stage: a title card, the product (captions on the
// left, Loudr's settings window on the right), a card about what is sent,
// and the sign-off. The cards are full layers above the product and the
// cursor, so a cut never shows the pointer.
//
// The script calls a.mark() at the start of every beat. The recorder writes
// down the frame each mark lands on, and the soundtrack is assembled from
// those frames, so narration and sound effects stay on the action however
// the timing is tuned here.
import { Check as Tick, ChevronDown, Music2 } from 'lucide-react'
import { Bar, Check, Collapse, Field, Fly, Menu, Popover, Spinner, Stage, States, Swap, Window, type Act } from '../kit'
import './film.css'

type Shot = 'title' | 'product' | 'trust' | 'outro'
type Busy = 'idle' | 'busy' | 'done'

type S = {
  shot: Shot
  caption: number
  menu: boolean
  provider: string
  keyOpen: boolean
  key: string
  focus: 'none' | 'key' | 'script'
  saving: Busy
  connected: boolean
  script: string
  playing: boolean
  exporting: Busy
  fly: boolean
  file: boolean
  trust: number
  signed: boolean
}

const initial: S = {
  shot: 'title',
  caption: 0,
  menu: false,
  provider: 'Off',
  keyOpen: false,
  key: '',
  focus: 'none',
  saving: 'idle',
  connected: false,
  script: '',
  playing: false,
  exporting: 'idle',
  fly: false,
  file: false,
  trust: 0,
  signed: false,
}

const PROVIDERS = ['Off', 'ElevenLabs', 'Google Gemini']
const SCRIPT = 'Welcome back to the channel. Today, three tools that changed how I work.'
const FILE = 'Loudr - Welcome back to the channel.mp3'

const CAPTIONS = [
  ['Bring your own voice.', 'Connect your ElevenLabs or Gemini account.'],
  ['Paste your key.', 'It never leaves this computer.'],
  ['Type it. Play it.', 'Your script, spoken like a real person.'],
  ['Save the MP3.', 'Ready for your next reel.'],
]
const TRUST = ['Only the script you play is sent.', 'Reading aloud stays offline.', 'Your key stays on your computer.']

/** Name a moment for the soundtrack. Each cue gets a number, so two cues
 *  with the same name in a row are still two events. */
let cues = 0
const cue = (a: Act<S>, name: string) => a.mark(`${name}#${cues++}`)

async function script(a: Act<S>) {
  // 1. Title card
  cue(a, 'title')
  await a.wait(2900)
  await a.set({ shot: 'product' })
  await a.wait(700)

  // 2. Choose a provider
  cue(a, 'connect')
  await a.wait(500)
  await a.click({ at: 'select', x: 0.7, y: 0.5 })
  cue(a, 'click')
  await a.set({ menu: true })
  await a.wait(500)
  await a.move({ at: 'prov-1', x: 0.35, y: 0.55 })
  await a.wait(300)
  await a.click()
  cue(a, 'click')
  await a.set({ menu: false, provider: 'ElevenLabs' })
  await a.wait(300)
  await a.set({ keyOpen: true })
  await a.wait(1500)

  // 3. Paste the key and save it
  cue(a, 'key')
  await a.set({ caption: 1 })
  await a.click({ at: 'key', x: 0.3, y: 0.5 })
  cue(a, 'click')
  await a.set({ focus: 'key' })
  await a.wait(350)
  await a.key('cmd+v')
  await a.set({ key: 'sk_3f9a7c21e84b05d6' })
  await a.wait(550)
  await a.click({ at: 'save-key', x: 0.5, y: 0.55 })
  cue(a, 'click')
  await a.set({ saving: 'busy', focus: 'none' })
  await a.wait(750)
  await a.set({ saving: 'done' })
  cue(a, 'done')
  await a.wait(550)
  await a.set({ keyOpen: false, connected: true })
  await a.wait(1100)

  // 4. Type a script and play it
  cue(a, 'play')
  await a.set({ caption: 2 })
  await a.click({ at: 'script', x: 0.2, y: 0.3 })
  cue(a, 'click')
  await a.set({ focus: 'script' })
  cue(a, 'typing')
  await a.type('script', SCRIPT, { interval: 26 })
  cue(a, 'typed')
  await a.wait(350)
  await a.click({ at: 'play', x: 0.5, y: 0.55 })
  cue(a, 'click')
  await a.set({ playing: true, focus: 'none' })
  cue(a, 'speaking')
  // Step off the button so "Stop" can be read
  await a.wait(350)
  await a.move({ at: 'script', x: 0.78, y: 1.75 })
  await a.wait(1900)

  // 5. Save it as a file
  cue(a, 'save')
  await a.set({ caption: 3 })
  await a.click({ at: 'save', x: 0.5, y: 0.55 })
  cue(a, 'click')
  await a.set({ exporting: 'busy', playing: false })
  await a.wait(800)
  await a.set({ exporting: 'done', fly: true })
  cue(a, 'whoosh')
  await a.wait(760)
  await a.set({ file: true })
  cue(a, 'done')
  await a.wait(2300)

  // 6. What is sent, and what is not
  cue(a, 'trust')
  await a.set({ shot: 'trust' })
  await a.wait(500)
  for (let line = 1; line <= TRUST.length; line++) {
    await a.set({ trust: line })
    cue(a, 'pop')
    await a.wait(900)
  }
  await a.wait(1100)

  // 7. Sign-off
  cue(a, 'outro')
  await a.set({ shot: 'outro' })
  await a.wait(350)
  await a.set({ signed: true })
  await a.wait(3900)
  cue(a, 'end')
  await a.wait(600)
}

export function Film() {
  return (
    <Stage<S>
      width={1280}
      height={720}
      initial={initial}
      script={script}
      play="once"
      theme="dark"
      keys="bottom-left"
      cursor={{ at: { x: 760, y: 560 }, style: 'large', idle: false }}
      className="film"
    >
      {(s, rect) => {
        const select = rect('select')
        return (
          <>
            {/* ---------------- Product shot ---------------- */}
            <div className="film-copy">
              <div className="film-eyebrow">
                <i />
                Studio voices · new in Loudr 0.4
              </div>
              <div className="film-caption">
                <Swap id={s.caption}>{CAPTIONS[s.caption][0]}</Swap>
              </div>
              <div className="film-sub">
                <Swap id={s.caption}>{CAPTIONS[s.caption][1]}</Swap>
              </div>

              <div className="film-wave" data-on={s.playing || undefined} aria-hidden>
                {Array.from({ length: 34 }, (_, i) => (
                  <i key={i} />
                ))}
              </div>

              <div className="film-file" data-target="file" data-show={s.file || undefined}>
                <span className="film-file__art">
                  <Music2 size={18} strokeWidth={2} />
                </span>
                <span className="film-file__text">
                  <span className="film-file__name">{FILE}</span>
                  <span className="film-file__meta">MP3 · 0:05 · saved to Downloads</span>
                </span>
                <Check on={s.file} size={18} />
              </div>
            </div>

            <Window title="Loudr" className="film-window" body={{ className: 'film-body', 'data-crop': true }}>
              <div className="film-tabs">
                <span>Dictation</span>
                <span data-on>Text to speech</span>
              </div>

              <div className="film-card film-composer">
                <div className="film-label">Script to audio</div>
                <div className="film-script" data-target="script" data-cursor="ibeam" data-focus={s.focus === 'script' || undefined}>
                  {s.script ? s.script : <span className="film-placeholder">Type or paste a script.</span>}
                  {s.focus === 'script' && <span className="film-caret" />}
                </div>
                <div className="film-actions">
                  <span className="film-button film-button--primary" data-target="play" data-cursor="pointer">
                    <States value={s.playing ? 'stop' : 'play'} states={{ play: 'Play', stop: 'Stop' }} />
                  </span>
                  <span className="film-button" data-target="save" data-cursor="pointer">
                    <States
                      value={s.exporting}
                      states={{
                        idle: 'Save as audio…',
                        busy: (
                          <span className="film-button__busy">
                            <Spinner size={13} /> Saving
                          </span>
                        ),
                        done: 'Save as audio…',
                      }}
                    />
                  </span>
                  <span className="film-speaking" data-on={s.playing || undefined}>
                    <i />
                    <i />
                    <i />
                    <i />
                    Speaking
                  </span>
                </div>
                <Collapse open={s.connected}>
                  <div className="film-hint">Spoken by ElevenLabs, with your account.</div>
                </Collapse>
              </div>

              <div className="film-card film-row">
                <span className="film-row__label">Studio voice</span>
                <span className="film-select" data-target="select" data-cursor="pointer" data-open={s.menu || undefined}>
                  <Swap id={s.provider}>{s.provider}</Swap>
                  <ChevronDown size={14} strokeWidth={2.25} />
                </span>
              </div>

              <Collapse open={s.keyOpen}>
                <div className="film-key">
                  <Field
                    className="film-field"
                    value={s.key}
                    password
                    focused={s.focus === 'key'}
                    placeholder="ElevenLabs API key"
                    box={{ 'data-target': 'key' }}
                  />
                  <span className="film-button film-button--primary" data-target="save-key" data-cursor="pointer">
                    <States
                      value={s.saving}
                      states={{
                        idle: 'Save key',
                        busy: <Spinner size={13} />,
                        done: <Tick size={15} strokeWidth={2.75} />,
                      }}
                    />
                  </span>
                </div>
              </Collapse>

              <Collapse open={s.connected}>
                <div className="film-card film-row film-row--stack">
                  <span className="film-row__label">Voice</span>
                  <span className="film-select film-select--quiet">
                    Rachel
                    <ChevronDown size={14} strokeWidth={2.25} />
                  </span>
                </div>
                <div className="film-card film-row film-row--stack">
                  <span className="film-row__label">Quality</span>
                  <span className="film-select film-select--quiet">
                    Multilingual v2
                    <ChevronDown size={14} strokeWidth={2.25} />
                  </span>
                </div>
              </Collapse>

              {/* The rest of the settings page: filler that runs off the frame */}
              <div className="film-card film-row film-row--filler">
                <span className="film-filler">
                  <Bar w={118} h={8} tone={3} />
                  <Bar w={186} h={6} tone={2} />
                </span>
                <Bar w={84} h={30} r={15} tone={1} />
              </div>
              <div className="film-card film-row film-row--filler">
                <span className="film-filler">
                  <Bar w={92} h={8} tone={3} />
                  <Bar w={150} h={6} tone={2} />
                </span>
                <Bar w={112} h={30} r={15} tone={1} />
              </div>
              <div className="film-card film-row film-row--filler">
                <span className="film-filler">
                  <Bar w={134} h={8} tone={3} />
                  <Bar w={170} h={6} tone={2} />
                </span>
                <Bar w={70} h={30} r={15} tone={1} />
              </div>
            </Window>

            <Popover
              show={s.menu}
              origin="top right"
              className="film-menu"
              style={{ left: select.x + select.width - 224, top: select.y + select.height + 6, width: 224 }}
            >
              <Menu
                items={PROVIDERS.map((name) => (
                  <span className="film-menu__item" key={name}>
                    <span className="film-menu__tick">{name === s.provider && <Tick size={13} strokeWidth={2.5} />}</span>
                    {name}
                  </span>
                ))}
                targets={PROVIDERS.map((_, i) => `prov-${i}`)}
              />
            </Popover>

            {!s.file && (
              <Fly from={rect('save')} to={rect('file')} go={s.fly} duration={720} className="film-fly">
                <span className="film-fly__chip">
                  <Music2 size={14} strokeWidth={2.25} /> MP3
                </span>
              </Fly>
            )}

            {/* ---------------- Cards ---------------- */}
            <div className="film-layer film-title" data-show={s.shot === 'title' || undefined}>
              <div className="film-title__bars" aria-hidden>
                {Array.from({ length: 21 }, (_, i) => (
                  <i key={i} />
                ))}
              </div>
              <div className="film-title__line">Some scripts need</div>
              <div className="film-title__line film-title__line--soft">a real voice.</div>
            </div>

            <div className="film-layer film-trust" data-show={s.shot === 'trust' || undefined}>
              <div className="film-trust__lines">
                {TRUST.map((line, i) => (
                  <div className="film-trust__line" key={line} data-show={s.trust > i || undefined}>
                    <Check on={s.trust > i} size={30} />
                    {line}
                  </div>
                ))}
              </div>
            </div>

            <div className="film-layer film-outro" data-show={s.shot === 'outro' || undefined} data-signed={s.signed || undefined}>
              <img className="film-outro__icon" src="/icon.png" alt="" />
              <div className="film-outro__name">Loudr</div>
              <div className="film-outro__line">Free. Open source. Yours to run.</div>
              <div className="film-outro__url">getloudr.vercel.app</div>
            </div>
          </>
        )
      }}
    </Stage>
  )
}
