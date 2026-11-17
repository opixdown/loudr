// Entry point. With ?record in the address, time is put under the
// recorder's control before React or the film load, so every frame of the
// video is rendered exactly once (see record.mjs).
import { installVirtualClock } from '../kit/clock'

const recording = new URLSearchParams(location.search).has('record')
if (recording) installVirtualClock({ fps: 30 })

const { start } = await import('./app')
await document.fonts.ready
start()
;(window as unknown as { __filmReady: boolean }).__filmReady = true
