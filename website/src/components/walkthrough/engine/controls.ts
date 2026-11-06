/**
 * The controls of a stage: the start button, play and pause, the step buttons, the scrub bar with a
 * tick per step (the moment's tick is gold), captions on and off, and fullscreen.
 *
 * Keys, while the stage has focus or fills the screen: Space or K plays and pauses, → plays the next
 * step and stops at its end, ← goes back one step, Home goes to the start, F is fullscreen, C turns
 * captions on and off, N opens the speaker-notes window (`notes.ts`), and `,` and `.` move one frame
 * while paused. In fullscreen the controls and the mouse pointer hide after two seconds without a
 * mouse move, so a screen recording shows the stage alone.
 */
import { bindNotes } from './notes'
import type { Stage } from './stage'

export function bindControls(stage: Stage): void {
  const root = stage.root
  const $ = <E extends Element = HTMLElement>(s: string) => root.querySelector<E & HTMLElement>(s)!
  const scrub = $('.wt-scrub')
  const fill = $('.wt-scrub-fill')
  const step = $('.wt-step')
  const beats = stage.tl.beats
  const started = () => (root.dataset.started = '')

  for (const b of beats) {
    const tick = document.createElement('i')
    tick.className = 'wt-tick'
    if (b.moment) tick.dataset.moment = ''
    tick.style.left = `${(b.end / stage.duration) * 100}%`
    scrub.append(tick)
  }

  const update = () => {
    fill.style.transform = `scaleX(${stage.t / stage.duration})`
    const i = stage.beatAt()
    const text = `<b>${i + 1} / ${beats.length}</b>&ensp;${beats[i]?.title ?? ''}`
    if (step.dataset.text !== text) {
      step.dataset.text = text
      step.innerHTML = text
    }
    scrub.setAttribute('aria-valuenow', String(Math.round((stage.t / stage.duration) * 100)))
  }
  stage.onUpdate(update)

  $('.wt-start').addEventListener('click', () => {
    started()
    stage.seek(0)
    stage.play()
    root.focus({ preventScroll: true })
  })
  $('.wt-play-i').addEventListener('click', () => {
    started()
    stage.toggle()
  })
  $('.wt-prev').addEventListener('click', () => {
    started()
    stage.prev()
  })
  $('.wt-next').addEventListener('click', () => {
    started()
    stage.next()
  })
  const cc = $('.wt-cc')
  const setCaptions = (on: boolean) => {
    root.dataset.captions = on ? 'on' : 'off'
    cc.setAttribute('aria-pressed', String(on))
  }
  cc.addEventListener('click', () => setCaptions(root.dataset.captions === 'off'))
  const full = () => {
    if (document.fullscreenElement) void document.exitFullscreen()
    else void root.requestFullscreen()
  }
  $('.wt-full').addEventListener('click', full)

  // Scrubbing pauses, and the frame follows the pointer.
  let dragging = false
  const seekTo = (e: PointerEvent) => {
    const r = scrub.getBoundingClientRect()
    stage.seek(((e.clientX - r.left) / r.width) * stage.duration)
  }
  scrub.addEventListener('pointerdown', (e) => {
    started()
    dragging = true
    scrub.setPointerCapture(e.pointerId)
    stage.pause()
    seekTo(e)
  })
  scrub.addEventListener('pointermove', (e) => dragging && seekTo(e))
  scrub.addEventListener('pointerup', () => (dragging = false))

  root.addEventListener('keydown', (e) => {
    if (e.altKey || e.ctrlKey || e.metaKey) return
    const k = e.key
    const handled = true
    if (k === ' ' || k === 'k') {
      started()
      stage.toggle()
    } else if (k === 'ArrowRight') {
      started()
      stage.next()
    } else if (k === 'ArrowLeft') {
      started()
      stage.prev()
    } else if (k === 'Home') {
      stage.pause()
      stage.seek(0)
    } else if (k === 'f') full()
    else if (k === 'c') setCaptions(root.dataset.captions === 'off')
    else if (k === '.' || k === ',') {
      stage.pause()
      stage.seek(stage.t + (k === '.' ? 1 : -1) / 60)
    } else return
    if (handled) e.preventDefault()
  })

  // In fullscreen, the controls and the mouse pointer hide when the mouse rests.
  let idle = 0
  const wake = () => {
    delete root.dataset.idle
    clearTimeout(idle)
    idle = window.setTimeout(() => (root.dataset.idle = ''), 2000)
  }
  root.addEventListener('pointermove', wake)
  document.addEventListener('fullscreenchange', () => {
    wake()
    if (document.fullscreenElement === root) root.focus({ preventScroll: true })
  })

  bindNotes(stage)
  update()
}
