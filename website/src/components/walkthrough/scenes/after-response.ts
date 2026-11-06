/**
 * The `afterResponse` walkthrough: the beats, built from the reusable parts.
 */
import { Stage } from '../engine/stage'
import { bindControls } from '../engine/controls'
import { Editor } from '../parts/editor'
import { Browser } from '../parts/browser'
import { Server } from '../parts/server'
import { Inbox } from '../parts/inbox'
import { Card, Pointer, Wire } from '../parts/movers'

/**
 * The brief. Every part, motion and caption below is judged against `moment`; what `leavesOut`
 * names is true and stays out on purpose, because it would pull the eye away from the moment.
 */
export const brief = {
  moment: 'I can do work after the browser has the page, without a second request.',
  expectation: 'When the page has loaded, the request is over.',
  proof: 'The page is done and the Network panel shows one request, then the receipt email arrives.',
  leavesOut: ['memory and CPU', 'limits and deadlines', 'failure cases', 'that it is not a job queue'],
  then: 'The Core\\Task::afterResponse reference page.',
}

export function start(root: HTMLElement): void {
  const stage = new Stage(root)
  const { tl, camera } = stage
  const wire = stage.part('wire', Wire)
  const browser = stage.part('browser', Browser)
  const inbox = stage.part('inbox', Inbox)
  const editor = stage.part('editor', Editor)
  const server = stage.part('server', Server)
  const card = stage.part('receipt', Card)
  const pointer = stage.part('pointer', Pointer)

  const lane = server.lane('POST /order')
  // While it waits, the card sits just under the growing bar.
  const underTip = (t: number) => {
    const p = lane.tip(t)
    return { x: p.x + 40, y: p.y + 92 }
  }
  const onTip = (t: number) => {
    const p = lane.tip(t)
    return { x: p.x - 30, y: p.y + 70 }
  }
  const mailbox = () => inbox.anchor('.wt-mails')
  let row: ReturnType<Browser['request']>

  tl.beat('The shop', { caption: 'A visitor buys a desk lamp in an online shop.' }, () => {
    tl.wait(0.2)
    browser.show('pop', 0)
    inbox.show('pop', 0.12)
    editor.show('pop', 0.24)
    server.show('pop', 0.36)
    // Wide, the server is to the right of the browser. On a phone it is below, so the wire loops down
    // the left margin.
    wire.connect(
      { wide: () => browser.edge('right', 0.45), tall: () => browser.edge('left', 0.45) },
      () => ({ x: server.rect().x, y: lane.origin(0).y }),
      { leave: { wide: 'right', tall: 'left' }, arrive: 'right', ends: [browser, server] },
    )
    tl.wait(2.4)
  })

  tl.beat('The code', { caption: 'This is the code of the order page. Look at the highlighted lines.' }, () => {
    camera.focus(editor, { maxZoom: 1.7 })
    tl.wait(0.9)
    editor.highlight(5, 8)
    tl.wait(0.5)
    editor.glow(1)
    tl.wait(2.6)
  })

  tl.beat('The order', { caption: 'The visitor clicks the button. The browser sends one request.' }, () => {
    editor.glow(0)
    editor.unhighlight()
    camera.focus(browser, { maxZoom: 1.45 })
    tl.wait(0.7)
    tl.wait(pointer.moveTo(browser.at('[data-anchor="order"]')))
    tl.wait(pointer.click())
    browser.loading(true)
    row = browser.request({ method: 'POST', path: '/order' })
    tl.wait(0.2)
    // On a phone the server is out of the shot, so the camera follows the request down to it.
    camera.focus(server, { only: 'tall' })
    tl.wait(wire.pulse('forward', 0.9))
    lane.start('runs the page')
    tl.wait(0.4)
  })

  tl.beat('The server', { caption: 'The server runs the code. The function goes into a list and waits.' }, () => {
    pointer.leave()
    camera.focus([editor, server], { maxZoom: 1.3, only: 'wide' })
    // On a phone the camera shows the code, then flies with the card down to the server.
    camera.focus(editor, { only: 'tall' })
    tl.wait(0.5)
    editor.highlight(2, 3)
    tl.wait(0.8)
    editor.highlight(6, 8)
    tl.wait(0.35)
    card.appear(editor.lines(6, 8, 380))
    tl.wait(0.6)
    camera.focus(server, { only: 'tall' })
    tl.wait(card.follow(underTip, 'soft', 0.1))
    tl.wait(0.5)
    editor.highlight(10)
    tl.wait(1.1)
  })

  tl.beat('The page', { caption: 'The browser has the page. The request is finished.' }, () => {
    editor.unhighlight()
    wire.pulse('back', 0.9, 'var(--wt-ok)')
    camera.focus(browser, { maxZoom: 1.45 })
    tl.wait(0.85)
    server.marker(lane, 'Browser has the page')
    lane.phase('sends the receipt', 'var(--wt-after)')
    row.finish(200, 38)
    browser.loading(false)
    tl.wait(browser.screen('done'))
    // Stillness: the viewer has to believe the story is over.
    tl.wait(1.8)
  })

  tl.beat(
    'Still running',
    {
      caption: 'But the same request is still running. It sends the receipt now.',
      notes: 'The moment. Let the pull-back land before you say anything.',
      moment: true,
    },
    () => {
      tl.slow(0.55)
      // On a phone the code stays out of the shot, so the three parts that show the moment are larger.
      camera.focus([browser, server, inbox], { only: 'tall' })
      tl.wait(camera.focus('all', { motion: 'glide', only: 'wide' }) * 0.4)
      card.glow(1)
      tl.wait(card.follow(onTip, 'soft', 0.06))
      tl.wait(1.6)
      tl.wait(card.moveTo(mailbox, 'soft', -0.18) * 0.7)
      inbox.arrive({ from: 'shop.example.com', subject: 'Receipt for order #1042', preview: 'Thank you for your order' })
      card.leave()
      lane.end()
      tl.wait(1.4)
    },
  )

  tl.beat('One request', { caption: 'The browser made one request. There was no second request.' }, () => {
    camera.focus(browser, { maxZoom: 1.45 })
    tl.wait(0.8)
    browser.highlightRow(0)
    tl.wait(2.4)
    tl.slow(1)
  })

  tl.beat('afterResponse', { caption: 'Core\\Task::afterResponse runs your function after the response is sent.' }, () => {
    browser.highlightRow(0, false)
    camera.focus('all', { only: 'wide' })
    camera.focus(editor, { only: 'tall' })
    tl.wait(0.4)
    editor.highlight(6, 8)
    editor.glow(1)
    tl.wait(3.2)
  })

  bindControls(stage)
  stage.start()
}
