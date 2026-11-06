/**
 * Inbox: a list of messages that arrive from outside the request, such as an email, a webhook or a
 * log line.
 *
 * - `arrive({ from, subject, preview })` slides a new message in at the top, pushes the others down,
 *   rings the window once and counts the badge up.
 */
import { css, Panel, type Stage } from '../engine/stage'
import type { Track } from '../engine/time'

const ITEM = 96

export class Inbox extends Panel {
  private readonly items: { el: HTMLElement; in: Track }[] = []
  private readonly ring: HTMLElement
  private readonly ringT = this.tl.track(0)
  private readonly badge: HTMLElement
  private readonly badgeT = this.tl.track(0)
  private readonly empty: HTMLElement
  private readonly box: HTMLElement

  constructor(stage: Stage, el: HTMLElement) {
    super(stage, el)
    this.ring = el.querySelector('.wt-ring')!
    this.badge = el.querySelector('.wt-badge')!
    this.empty = el.querySelector('.wt-empty')!
    this.box = el.querySelector('.wt-mails')!
  }

  arrive(msg: { from: string; subject: string; preview?: string }): number {
    const el = document.createElement('div')
    el.className = 'wt-mail'
    el.innerHTML = '<div class="wt-avatar"></div><div><b></b><small></small></div>'
    el.querySelector('.wt-avatar')!.textContent = msg.from.slice(0, 1).toUpperCase()
    el.querySelector('b')!.textContent = msg.subject
    el.querySelector('small')!.textContent = msg.preview ? `${msg.from} · ${msg.preview}` : msg.from
    this.box.append(el)
    const t = this.tl.track(0)
    const d = t.to(1, 'pop')
    this.items.push({ el, in: t })
    this.ringT.set(0)
    this.ringT.to(1, 'linear', { dur: 1.1 })
    this.badgeT.to(this.items.length, 'pop')
    return d
  }

  render(t: number): void {
    this.renderPanel(t)
    // The newest message is on top; each older one sits one row lower per newer one that has arrived.
    let below = 0
    for (let i = this.items.length - 1; i >= 0; i--) {
      const it = this.items[i]
      const p = it.in.value(t)
      css(it.el, 'opacity', String(Math.min(1, Math.max(0, p))))
      css(it.el, 'transform', `translateY(${14 + below * ITEM - (1 - p) * 40}px) scale(${0.92 + 0.08 * p})`)
      below += Math.min(1, Math.max(0, p))
    }
    css(this.empty, 'opacity', String(Math.max(0, 1 - below * 3)))
    const r = this.ringT.value(t)
    css(this.ring, 'opacity', String(r > 0 && r < 1 ? 1 - r : 0))
    css(this.ring, 'transform', `scale(${1 + r * 0.06})`)
    const b = this.badgeT.value(t)
    const n = Math.round(Math.max(0, b))
    if (this.badge.textContent !== String(n)) this.badge.textContent = String(n)
    const pop = b - Math.floor(b + 0.001)
    css(this.badge, 'opacity', b > 0.05 ? '1' : '0')
    css(this.badge, 'transform', `scale(${b > 0.05 ? 1 + Math.abs(pop) * 0.4 : 0.5})`)
  }
}
