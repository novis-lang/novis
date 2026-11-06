/**
 * The parts gallery: every reusable part on a stage of its own, one step per state, so a change to a
 * part is seen in every state it has, and in the motion between them. Each specimen calls only the
 * part's public API, the way a scene does. A new part or a new state gets a specimen or a step here.
 *
 * `bun nv walkthrough --frames in-depth/walkthrough-parts` takes a picture of every step of every
 * specimen.
 */
import { Stage, type Layout, type Point, type Rect } from '../engine/stage'
import { bindControls } from '../engine/controls'
import { Editor } from '../parts/editor'
import { Browser } from '../parts/browser'
import { Server } from '../parts/server'
import { Inbox } from '../parts/inbox'
import { Card, Pointer, Wire } from '../parts/movers'

/** A stage for the specimen in `#id`; `author` writes its steps, then the player starts. */
function specimen(id: string, author: (stage: Stage) => void): void {
  const stage = new Stage(document.getElementById(id)!)
  author(stage)
  bindControls(stage)
  stage.start()
}

/** The region the movers' specimens frame, per layout, and a point in it by fractions of its size. */
const REGION: Record<Layout, Rect> = {
  wide: { x: 300, y: 200, w: 1000, h: 500 },
  tall: { x: 50, y: 450, w: 800, h: 700 },
}
const at = (stage: Stage, fx: number, fy: number) => (): Point => {
  const r = REGION[stage.layout]
  return { x: r.x + r.w * fx, y: r.y + r.h * fy }
}
function frameRegion(stage: Stage): void {
  stage.camera.focus(REGION.wide, { motion: 'step', only: 'wide', pad: 0 })
  stage.camera.focus(REGION.tall, { motion: 'step', only: 'tall', pad: 0 })
}

export function startGallery(): void {
  specimen('wt-part-editor', (stage) => {
    const { tl, camera } = stage
    const editor = stage.part('editor', Editor)
    camera.focus(editor, { motion: 'step' })
    tl.beat('show()', { caption: 'The window pops in with the code.' }, () => {
      tl.wait(0.2)
      tl.wait(editor.show('pop') + 0.6)
    })
    tl.beat('highlight(2, 3)', { caption: 'The first highlight appears in place.' }, () => {
      tl.wait(editor.highlight(2, 3) + 0.8)
    })
    tl.beat('highlight(6, 8)', { caption: 'A later highlight slides to its lines and takes their height.' }, () => {
      tl.wait(editor.highlight(6, 8) + 0.8)
    })
    tl.beat('glow(1)', { caption: 'The highlight shines, for the lines the step is about.' }, () => {
      tl.wait(editor.glow(1) + 0.8)
    })
    tl.beat('unhighlight()', { caption: 'The highlight fades out.' }, () => {
      tl.wait(editor.unhighlight() + 0.8)
    })
    tl.beat('clear() and type(1, 10)', { caption: 'The code is typed in again, line by line, with a caret.' }, () => {
      editor.clear()
      tl.wait(editor.type(1, 10) + 0.8)
    })
  })

  specimen('wt-part-browser', (stage) => {
    const { tl, camera } = stage
    const browser = stage.part('browser', Browser)
    camera.focus(browser, { motion: 'step' })
    let row: ReturnType<Browser['request']>
    tl.beat('show()', { caption: 'The first screen of the page is shown.' }, () => {
      tl.wait(0.2)
      tl.wait(browser.show('pop') + 0.6)
    })
    tl.beat('loading(true) and request()', { caption: 'The tab spins and a pending row counts its time up.' }, () => {
      browser.loading(true)
      row = browser.request({ method: 'POST', path: '/order' })
      tl.wait(1.6)
    })
    tl.beat('finish(200, 38)', { caption: 'The row ends with its status and time, and the tab stops.' }, () => {
      row.finish(200, 38)
      browser.loading(false)
      tl.wait(1)
    })
    tl.beat("screen('done')", { caption: 'The page cross-fades to the next screen, one element after another.' }, () => {
      tl.wait(browser.screen('done') + 0.6)
    })
    tl.beat('a second request()', { caption: 'A new row is added under the first.' }, () => {
      const second = browser.request({ method: 'GET', path: '/receipt' })
      tl.wait(0.7)
      second.finish(200, 12)
      tl.wait(0.8)
    })
    tl.beat('highlightRow(0)', { caption: 'A ring marks the first row.' }, () => {
      tl.wait(browser.highlightRow(0) + 0.8)
    })
  })

  specimen('wt-part-server', (stage) => {
    const { tl, camera } = stage
    const server = stage.part('server', Server)
    camera.focus(server, { motion: 'step' })
    const lane = server.lane('POST /order')
    const other = server.lane('GET /')
    tl.beat('show() and lane.start()', { caption: 'A request starts. Its bar grows while it runs.' }, () => {
      tl.wait(0.2)
      tl.wait(server.show('pop'))
      lane.start('runs the page')
      tl.wait(1.6)
    })
    tl.beat('lane.phase()', { caption: 'The next part of the request starts where the bar is.' }, () => {
      lane.phase('sends the receipt', 'var(--wt-after)')
      tl.wait(1.2)
    })
    tl.beat('a second lane', { caption: 'A second request starts later, so its bar starts further right.' }, () => {
      other.start('runs the page')
      tl.wait(1.2)
    })
    tl.beat('marker()', { caption: 'A dashed line marks one time on every lane, with a label.' }, () => {
      tl.wait(server.marker(lane, 'Browser has the page') + 0.8)
    })
    tl.beat('lane.end()', { caption: 'Each request ends with a check mark.' }, () => {
      other.end()
      tl.wait(0.5)
      tl.wait(lane.end() + 0.8)
    })
  })

  specimen('wt-part-inbox', (stage) => {
    const { tl, camera } = stage
    const inbox = stage.part('inbox', Inbox)
    camera.focus(inbox, { motion: 'step' })
    tl.beat('show()', { caption: 'The window shows that it has no messages.' }, () => {
      tl.wait(0.2)
      tl.wait(inbox.show('pop') + 0.6)
    })
    tl.beat('arrive()', { caption: 'A message slides in, the window rings and the badge counts up.' }, () => {
      tl.wait(inbox.arrive({ from: 'shop.example.com', subject: 'Receipt for order #1042', preview: 'Thank you for your order' }) + 0.8)
    })
    tl.beat('a second arrive()', { caption: 'The newest message is on top and pushes the others down.' }, () => {
      tl.wait(inbox.arrive({ from: 'shop.example.com', subject: 'Your order has shipped', preview: 'It arrives on Friday' }) + 0.8)
    })
  })

  specimen('wt-part-wire', (stage) => {
    const { tl } = stage
    const wire = stage.part('wire', Wire)
    const loop = stage.part('loop', Wire)
    frameRegion(stage)
    tl.beat('connect()', { caption: 'A wire is drawn between two points. Both ends point to the right.' }, () => {
      tl.wait(0.2)
      tl.wait(wire.connect(at(stage, 0.05, 0.15), at(stage, 0.95, 0.45)) + 0.4)
    })
    tl.beat("pulse('forward')", { caption: 'A dot travels along the wire, from the start to the end.' }, () => {
      tl.wait(wire.pulse('forward', 0.9) + 0.4)
    })
    tl.beat("pulse('back')", { caption: 'A dot in another colour travels back.' }, () => {
      tl.wait(wire.pulse('back', 0.9, 'var(--wt-ok)') + 0.4)
    })
    tl.beat("leave: 'left'", { caption: 'An end that sets off the other way makes a loop.' }, () => {
      tl.wait(loop.connect(at(stage, 0.3, 0.7), at(stage, 0.3, 0.95), { leave: 'left', arrive: 'right' }) + 0.4)
      tl.wait(loop.pulse('forward', 0.9) + 0.4)
    })
    tl.beat('disconnect()', { caption: 'The wires are taken away.' }, () => {
      wire.disconnect()
      tl.wait(loop.disconnect() + 0.6)
    })
  })

  specimen('wt-part-movers', (stage) => {
    const { tl } = stage
    const card = stage.part('card', Card)
    const pointer = stage.part('pointer', Pointer)
    frameRegion(stage)
    tl.beat('card.appear()', { caption: 'A card pops in at a point.' }, () => {
      tl.wait(0.2)
      tl.wait(card.appear(at(stage, 0.25, 0.3)) + 0.6)
    })
    tl.beat('card.glow(1) and label()', { caption: 'The card shines, and its text changes.' }, () => {
      card.glow(1)
      card.label('sends the receipt')
      tl.wait(1)
    })
    tl.beat('card.moveTo()', { caption: 'The card flies to another point along a slight arc.' }, () => {
      tl.wait(card.moveTo(at(stage, 0.75, 0.3)) + 0.6)
    })
    tl.beat('card.leave()', { caption: 'The card fades out.' }, () => {
      tl.wait(card.leave() + 0.6)
    })
    tl.beat('pointer.moveTo()', { caption: 'The pointer comes in from below and glides to a point.' }, () => {
      tl.wait(pointer.moveTo(at(stage, 0.5, 0.6)) + 0.6)
    })
    tl.beat('pointer.click()', { caption: 'The pointer presses, and a ring shows the click.' }, () => {
      tl.wait(pointer.click() + 0.6)
    })
    tl.beat('pointer.leave()', { caption: 'The pointer fades out.' }, () => {
      tl.wait(pointer.leave() + 0.6)
    })
  })
}
