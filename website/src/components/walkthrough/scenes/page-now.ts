/**
 * The `later` and `slotted` walkthrough, "Page now, slow parts later": the beats, built from the
 * reusable parts. The same request runs twice. First the page waits for its slowest part; then one
 * line, `Core\Response::slotted();`, is added and the page arrives at once, its parts after it.
 */
import type { Brief } from '../engine/brief'
import { Stage } from '../engine/stage'
import { bindControls } from '../engine/controls'
import { Editor } from '../parts/editor'
import { Browser, type Row } from '../parts/browser'
import { Server, type Lane } from '../parts/server'
import { Wire } from '../parts/movers'

/**
 * The brief. Every part, motion and caption below is judged against `moment`; what `leavesOut`
 * names is true and stays out on purpose, because it would pull the eye away from the moment.
 */
export const brief: Brief = {
  moment: 'The browser shows my page before its slow parts are ready, and they arrive in the same request.',
  expectation: 'A page with slow parts shows nothing until its slowest part is ready.',
  proof:
    'The page shows with two placeholders while both parts still run on the server. The Network panel shows one request, still open, until the last part fills its place.',
  leavesOut: [
    'a part that fails, and its error markup',
    'the deadline option',
    'the route form, slotted: true on #[Core\\Route]',
    'a later call inside a later part',
    'memory and limits',
    'the script that puts each part in place, and a browser without JavaScript',
    'that a part cannot set a header or the status',
  ],
  then: 'The Core\\Html::later and Core\\Response::slotted reference pages.',
}

/**
 * Scene seconds each part runs, in about the ratio of the snippet's 900 ms and 500 ms. A caption's
 * hold moves scene time on too, so the page of the second run arrives, and its moment is read, before
 * the faster part ends. Every run's main code ends `MAIN` after its parts start, so the two runs
 * compare.
 */
const POSTS = 8
const DEALS = 4.5
/** Scene seconds the main code runs before it ends: long enough for its bar to hold its label. */
const MAIN = 1.3

export function start(root: HTMLElement): void {
  const stage = new Stage(root)
  const { tl, camera } = stage
  const wire = stage.part('wire', Wire)
  const browser = stage.part('browser', Browser)
  const editor = stage.part('editor', Editor)
  const server = stage.part('server', Server)

  // The snippet's line numbers: 4 is `Core\Response::slotted();`, 5-8 and 9-12 the two parts, 13 the echo.
  const SLOTTED = 4
  const posts = [5, 8] as const
  const deals = [9, 12] as const
  const ECHO = 13

  // The left edge of the editor, level with some lines.
  const linesLeft = (from: number, to: number) => (t: number) => ({ x: editor.rect().x, y: editor.lines(from, to)(t).y })
  const fill = (name: string, where: 'right' | 'top' | 'bottom') => browser.at(`[data-fill="${name}"]`, where)

  // Until the scene adds it, the line that sends the page first is not in the file.
  editor.fold(SLOTTED)

  /** One run of the request: the lanes, the row in the Network panel and when the parts started. */
  interface Run {
    main: Lane
    posts: Lane
    deals: Lane
    row: Row
    /** When the two parts started, in scene time. */
    t0: number
  }

  /** The visitor opens the page: the row, the pulse, and the main code starting on the server. */
  const open = (): Pick<Run, 'main' | 'row'> => {
    browser.loading(true)
    const row = browser.request({ method: 'GET', path: '/' })
    tl.wait(0.15)
    camera.focus(server, { only: 'tall' })
    tl.wait(wire.pulse('forward', 0.9))
    const main = server.lane('GET /')
    main.start('main code')
    return { main, row }
  }

  /** Both parts start as children of the request, while the main code runs on. */
  const parts = (): Pick<Run, 'posts' | 'deals' | 't0'> => {
    tl.wait(0.15)
    const t0 = tl.now
    const p = server.lane('posts', { child: true })
    p.start('posts', 'var(--wt-after)')
    tl.wait(0.12)
    const d = server.lane('deals', { child: true })
    d.start('deals', 'var(--wt-after)')
    return { posts: p, deals: d, t0 }
  }

  /** Waits until scene time `at`, if the cursor is before it. */
  const until = (at: number) => tl.wait(Math.max(0, at - tl.now))

  let run: Run

  tl.beat(
    'The page',
    {
      caption: 'This page has two slow parts. Core\\Html::later builds each one.',
      notes: 'Lines 5 to 12 are the two parts. Each one waits a little, like a slow database query.',
    },
    () => {
      tl.wait(0.2)
      browser.show('pop', 0)
      editor.show('pop', 0.12)
      server.show('pop', 0.24)
      // Wide, the server is to the right of the browser. On a phone it is below, so the wire loops down
      // the left margin.
      wire.connect(
        { wide: () => browser.edge('right', 0.5), tall: () => browser.edge('left', 0.45) },
        (t) => run?.main.origin(t) ?? { x: server.rect().x, y: server.rect().y + 150 },
        { leave: { wide: 'right', tall: 'left' }, arrive: 'right', ends: [browser, server] },
      )
      tl.wait(0.8)
      camera.focus(editor, { maxZoom: 1.6, room: { wide: { left: 380 }, tall: {} } })
      tl.wait(0.9)
      editor.highlight(posts[0], deals[1])
      tl.wait(0.4)
      stage.say(editor, {
        side: { wide: 'left', tall: 'below' },
        point: { wide: linesLeft(posts[0], deals[1]), tall: () => editor.edge('bottom') },
      })
      tl.wait(0.8)
    },
  )

  tl.beat(
    'The request',
    {
      caption: 'A visitor opens the page. The browser sends one request.',
      notes: 'The dot on the wire is the request. Count it: one.',
    },
    () => {
      editor.unhighlight()
      camera.focus('all', { only: 'wide' })
      camera.focus(browser, { only: 'tall' })
      tl.wait(0.8)
      stage.say(browser, {
        side: { wide: 'right', tall: 'above' },
        point: { wide: browser.at('.wt-rows', 'right'), tall: () => browser.edge('top') },
        focus: [browser, server],
      })
      const opened = open()
      run = { ...opened, posts: null!, deals: null!, t0: 0 }
    },
  )

  tl.beat(
    'Both parts',
    {
      caption: 'Both parts run at the same time as the main code. They are part of the same request.',
      notes: 'Two thin bars under the request. They start together, and the main code goes on.',
    },
    () => {
      camera.focus([editor, server], { maxZoom: 1.3, room: { left: 360 }, only: 'wide' })
      camera.focus(server, { only: 'tall' })
      editor.highlight(posts[0], posts[1])
      Object.assign(run, parts())
      tl.wait(0.25)
      editor.highlight(deals[0], deals[1])
      until(run.t0 + MAIN - 0.15)
      editor.highlight(ECHO)
      until(run.t0 + MAIN)
      run.main.phase('waits for the parts', 'var(--wt-faint)')
      tl.wait(0.2)
      // On a phone the server fills the bottom of the shot, so the caption sits above the whole window.
      stage.say(server, {
        side: 'above',
        point: {
          wide: (t) => {
            const p = run.posts.tip(t)
            return { x: p.x - 20, y: p.y - 20 }
          },
          tall: () => server.edge('top'),
        },
        lift: 40,
        focus: [editor, server],
      })
      tl.wait(0.3)
    },
  )

  tl.beat(
    'It waits',
    {
      caption: 'The page waits for the slowest part. Until then, the visitor sees an empty page.',
      notes: 'Let the wait be felt. The spinner turns and the time in the Network panel counts up.',
    },
    () => {
      editor.unhighlight()
      // The empty page and the bars that keep it empty are in one shot, on a phone too.
      camera.focus([browser, server])
      until(run.t0 + DEALS)
      run.deals.end()
      tl.wait(0.3)
      // One part is done, and the page is still empty: the caption is read while the other part runs.
      stage.say(browser, {
        side: { wide: 'right', tall: 'below' },
        point: { wide: () => browser.edge('right', 0.3), tall: () => browser.edge('bottom') },
        focus: [browser, server],
      })
      until(run.t0 + POSTS)
      run.posts.end()
      tl.wait(0.15)
      server.marker(run.main, 'Browser has the page')
      run.main.end()
      tl.wait(wire.pulse('back', 0.9, 'var(--wt-ok)'))
      browser.filled('posts')
      browser.filled('deals')
      run.row.finish(200, 912)
      browser.loading(false)
      tl.wait(browser.screen('home'))
      tl.wait(0.8)
    },
  )

  tl.beat(
    'One line',
    {
      caption: 'Now add one line: Core\\Response::slotted(). The two parts stay the same.',
      notes: 'The only change to the file. The parts are not touched.',
    },
    () => {
      camera.focus(editor, { maxZoom: 1.6, room: { wide: { left: 380 }, tall: {} } })
      browser.clearRows()
      browser.screen('blank')
      server.clear()
      tl.wait(0.9)
      tl.wait(editor.insert(SLOTTED))
      editor.highlight(SLOTTED)
      editor.glow(1)
      stage.say(editor, {
        side: { wide: 'left', tall: 'below' },
        point: { wide: linesLeft(SLOTTED, SLOTTED), tall: () => editor.edge('bottom') },
      })
      tl.wait(0.8)
    },
  )

  tl.beat(
    'Again',
    {
      caption: 'The visitor opens the page again. The same request starts.',
      notes: 'Everything starts as before. Watch the browser.',
    },
    () => {
      editor.glow(0)
      editor.unhighlight()
      browser.unfill('posts')
      browser.unfill('deals')
      camera.focus('all', { only: 'wide' })
      camera.focus(browser, { only: 'tall' })
      tl.wait(0.7)
      // The caption is read before the request starts, so the two runs take the same scene time.
      stage.say(browser, {
        side: { wide: 'right', tall: 'above' },
        point: { wide: () => browser.edge('right', 0.25), tall: () => browser.edge('top') },
        focus: [browser, server],
      })
      const opened = open()
      run = { ...opened, ...parts() }
      until(run.t0 + MAIN)
    },
  )

  tl.beat(
    'Page now',
    {
      caption: 'The page arrives as soon as the main code ends. The two slow parts are still running.',
      notes: 'The moment. The page is there, and both bars are still growing. Let it land.',
      moment: true,
    },
    () => {
      tl.slow(0.5)
      run.main.phase('sends the parts', 'var(--wt-after)')
      server.marker(run.main, 'Browser has the page')
      camera.focus([browser, server], { only: 'tall' })
      tl.wait(wire.pulse('back', 0.7, 'var(--wt-ok)'))
      browser.screen('home')
      tl.wait(0.5)
      // On a phone the caption sits above the browser, over the code, so the page and the Network
      // panel stay in view.
      stage.say(browser, {
        side: { wide: 'right', tall: 'above' },
        point: { wide: fill('posts', 'right'), tall: () => browser.edge('top') },
        focus: [browser, server],
      })
      tl.wait(0.4)
    },
  )

  tl.beat(
    'The parts',
    {
      caption: 'Each part takes its place when it is ready. The faster part comes first.',
      notes: 'Deals first, then posts: the order they finish in, not the order on the page.',
    },
    () => {
      tl.slow(1)
      until(run.t0 + DEALS)
      run.deals.end()
      tl.wait(wire.pulse('back', 0.6, 'var(--wt-after)'))
      browser.fill('deals')
      stage.say(browser, {
        side: { wide: 'right', tall: 'above' },
        point: { wide: fill('deals', 'right'), tall: () => browser.edge('top') },
        focus: [browser, server],
      })
      until(run.t0 + POSTS)
      run.posts.end()
      tl.wait(wire.pulse('back', 0.6, 'var(--wt-after)'))
      browser.fill('posts')
      run.main.end()
      run.row.finish(200, 915)
      browser.loading(false)
      tl.wait(1)
    },
  )

  tl.beat(
    'One request',
    {
      caption: 'The browser made one request. Both parts came in the same response.',
      notes: 'Point at the one row in the Network panel. No second request, no extra script to write.',
    },
    () => {
      camera.focus(browser, { maxZoom: 1.4, room: { wide: { right: 300 }, tall: {} } })
      tl.wait(0.8)
      browser.highlightRow(0)
      stage.say(browser, {
        side: { wide: 'right', tall: 'below' },
        point: { wide: browser.at('.wt-row', 'right'), tall: browser.at('.wt-row', 'bottom') },
      })
      tl.wait(0.8)
    },
  )

  tl.beat(
    'later and slotted',
    {
      caption: 'Core\\Html::later runs the slow parts. Core\\Response::slotted() sends the page first.',
      notes: 'Name both once more. The reference pages come next.',
    },
    () => {
      browser.highlightRow(0, false)
      camera.focus(editor, { maxZoom: 1.6, room: { wide: { left: 380 }, tall: {} } })
      tl.wait(0.5)
      editor.highlight(SLOTTED, deals[1])
      editor.glow(1)
      stage.say(editor, {
        side: { wide: 'left', tall: 'below' },
        point: { wide: linesLeft(SLOTTED, deals[1]), tall: () => editor.edge('bottom') },
      })
      tl.wait(1.4)
    },
  )

  bindControls(stage)
  stage.start()
}
