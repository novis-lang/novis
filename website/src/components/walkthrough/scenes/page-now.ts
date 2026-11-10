/**
 * The `later` and `slotted` walkthrough, "Page now, slow parts later": the beats, built from the
 * reusable parts. The same page runs twice. First the usual way: the page arrives with empty places,
 * then a script loads and fetches each slow part from an API route of its own. Then the code of each
 * route moves into the page inside `Core\Html::later`, the script and the routes are deleted,
 * `Core\Response::slotted();` is added, and the page and its parts come in one request.
 */
import type { Brief } from '../engine/brief'
import { Stage } from '../engine/stage'
import { bindControls } from '../engine/controls'
import { Editor } from '../parts/editor'
import { Browser, type Row } from '../parts/browser'
import { Server, type Lane } from '../parts/server'
import { Card, Wire } from '../parts/movers'

/**
 * The brief. Every part, motion and caption below is judged against `moment`; what `leavesOut`
 * names is true and stays out on purpose, because it would pull the eye away from the moment.
 */
export const brief: Brief = {
  moment:
    'The browser shows my page at once, and the slow parts follow in the same response. I wrote no script and no API route, and there was no second request.',
  expectation:
    'To show a page fast while its slow parts load, I need a script that fetches them, an API route for each part, and more requests after the page.',
  proof:
    'The usual run has four files and four rows in the Network panel, and its slow parts start on the server only after the page and the script reached the browser. The Novis run has one file and one row, its slow parts start with the request, and they end before the faded bars of the usual run.',
  leavesOut: [
    'a part that fails, and its error markup',
    'the deadline option',
    'the route form, slotted: true on #[Core\\Route]',
    'a later call inside a later part',
    'memory and limits',
    'the script Novis sends that puts each part in place, and a browser without JavaScript',
    'that a part cannot set a header or the status',
    'CORS, CSRF and caching for the API routes of the usual way',
    'other frameworks that also stream a page',
  ],
  then: 'The Core\\Html::later and Core\\Response::slotted reference pages.',
}

/**
 * Scene seconds each slow part runs, in about the ratio of the snippets' 900 ms and 500 ms. Both runs
 * use the same figures, so the only difference between them is when the parts start. A caption's
 * hold moves scene time on too, so the page of the second run arrives, and its moment is read, before
 * the faster part ends.
 */
const POSTS = 8
const DEALS = 4.5
/** Scene seconds the main code of the page runs: long enough for its bar to hold its label. */
const MAIN = 1.3

/** The editor's files, in the order of `src` in `PageNow.astro`. */
const POSTS_ROUTE = 1
const DEALS_ROUTE = 2
const SCRIPT = 3
const NOVIS = 4

export function start(root: HTMLElement): void {
  const stage = new Stage(root)
  const { tl, camera } = stage
  const wire = stage.part('wire', Wire)
  const browser = stage.part('browser', Browser)
  const editor = stage.part('editor', Editor)
  const server = stage.part('server', Server)
  const card = stage.part('card', Card)

  // The usual page's lines: 2 checks the login, 3-6 write the page with its empty places and the script.
  const page = [3, 6] as const
  // An API route's lines: 2 checks the login, 3 writes the part.
  const CHECK = 2
  const BODY = 3
  // The Novis page's lines: 3 is `Core\Response::slotted();`, 4-7 and 8-11 the two parts.
  const SLOTTED = 3
  const posts = [4, 7] as const
  const deals = [8, 11] as const

  // The left edge of the editor, level with some lines of the current file.
  const linesLeft = (from: number, to: number) => (t: number) => ({ x: editor.rect().x, y: editor.lines(from, to)(t).y })
  const fill = (name: string, where: 'right' | 'top' | 'bottom') => browser.at(`[data-fill="${name}"]`, where)
  /** The editor's shot, with room for a caption on its left. */
  const onEditor = () => camera.focus(editor, { maxZoom: 1.6, room: { wide: { left: 380 }, tall: {} } })

  // Until the scene adds them, the Novis page has only its first, second and last line.
  editor.fold(SLOTTED, deals[1], NOVIS)

  /** The main request of a run, its row in the Network panel, and its parts. */
  interface Run {
    main: Lane
    row: Row
    posts: Lane
    deals: Lane
    postsRow?: Row
    dealsRow?: Row
    /** When the two parts started, in scene time. */
    t0: number
  }
  let run: Run

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

  /** Both parts start on the server, under the main lane, `label` naming each. */
  const parts = (label: (part: string) => string): Pick<Run, 'posts' | 'deals' | 't0'> => {
    const t0 = tl.now
    const p = server.lane(label('posts'), { child: true })
    p.start(label('posts'), 'var(--wt-after)')
    tl.wait(0.12)
    const d = server.lane(label('deals'), { child: true })
    d.start(label('deals'), 'var(--wt-after)')
    return { posts: p, deals: d, t0 }
  }

  /** Waits until scene time `at`, if the cursor is before it. */
  const until = (at: number) => tl.wait(Math.max(0, at - tl.now))

  tl.beat(
    'The usual way',
    {
      caption: 'The usual way to load slow parts later: a page with empty places, an API route for each part, and a script.',
      notes: 'Four tabs: the page, two API routes and a script. This is how most sites do it today.',
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
      onEditor()
      tl.wait(0.9)
      editor.highlight(page[0], page[1])
      tl.wait(0.4)
      stage.say(editor, {
        side: { wide: 'left', tall: 'below' },
        point: { wide: linesLeft(page[0], page[1]), tall: () => editor.edge('bottom') },
      })
      tl.wait(0.8)
    },
  )

  tl.beat(
    'The script',
    {
      caption: 'This script runs after the page has loaded. It fetches each part and puts it into the page.',
      notes: 'Code you write and keep in step with the page: the ids, the URLs, the loading text.',
    },
    () => {
      editor.unhighlight()
      tl.wait(editor.file(SCRIPT))
      editor.highlight(1, 7)
      tl.wait(0.4)
      stage.say(editor, {
        side: { wide: 'left', tall: 'below' },
        point: { wide: linesLeft(1, 7), tall: () => editor.edge('bottom') },
      })
      tl.wait(0.6)
    },
  )

  tl.beat(
    'The routes',
    {
      caption: 'Each part needs its own API route. Each route is a public URL, so it must check the login again.',
      notes: 'Forget the check in one route, and anybody can read that part.',
    },
    () => {
      editor.unhighlight()
      tl.wait(editor.file(POSTS_ROUTE))
      editor.highlight(CHECK)
      tl.wait(0.4)
      stage.say(editor, {
        side: { wide: 'left', tall: 'below' },
        point: { wide: linesLeft(CHECK, CHECK), tall: () => editor.edge('bottom') },
      })
      tl.wait(0.6)
    },
  )

  tl.beat(
    'The page',
    {
      caption: 'The page arrives quickly, but only with empty places. Then the browser loads the script.',
      notes: 'The first row ends fast. Nothing slow has started yet.',
    },
    () => {
      editor.unhighlight()
      camera.focus('all', { only: 'wide' })
      camera.focus(browser, { only: 'tall' })
      tl.wait(0.6)
      const opened = open()
      run = { ...opened, posts: null!, deals: null!, t0: 0 }
      tl.wait(MAIN)
      server.marker(run.main, 'Browser has the page')
      run.main.end()
      run.row.finish(200, 95)
      camera.focus(browser, { only: 'tall' })
      tl.wait(wire.pulse('back', 0.7, 'var(--wt-ok)'))
      browser.screen('home')
      tl.wait(0.5)
      stage.say(browser, {
        side: { wide: 'right', tall: 'above' },
        point: { wide: fill('posts', 'right'), tall: () => browser.edge('top') },
        focus: [browser, server],
      })
      const script = browser.request({ method: 'GET', path: '/home.js' })
      tl.wait(0.35)
      script.finish(200, 12)
    },
  )

  tl.beat(
    'Only now',
    {
      caption: 'The slow parts start only now. First the page and the script had to reach the browser.',
      notes: 'Two more requests. Look how far right the two bars start.',
    },
    () => {
      camera.focus([browser, server])
      run.postsRow = browser.request({ method: 'GET', path: '/api/posts' })
      tl.wait(0.12)
      run.dealsRow = browser.request({ method: 'GET', path: '/api/deals' })
      tl.wait(wire.pulse('forward', 0.7))
      Object.assign(run, parts((part) => `GET /api/${part}`))
      tl.wait(0.4)
      stage.say(server, {
        side: 'above',
        point: {
          wide: (t) => {
            const p = run.posts.origin(t)
            return { x: p.x + run.posts.x(run.t0), y: p.y - 20 }
          },
          tall: () => server.edge('top'),
        },
        lift: 40,
        focus: [browser, server],
      })
      until(run.t0 + DEALS)
      run.deals.end()
      tl.wait(wire.pulse('back', 0.6, 'var(--wt-after)'))
      browser.fill('deals')
      run.dealsRow!.finish(200, 503)
      until(run.t0 + POSTS)
      run.posts.end()
      tl.wait(wire.pulse('back', 0.6, 'var(--wt-after)'))
      browser.fill('posts')
      run.postsRow!.finish(200, 905)
      browser.loading(false)
      tl.wait(0.8)
    },
  )

  tl.beat(
    'The count',
    {
      caption: 'Four files and four requests. The login was checked three times.',
      notes: 'The cost of the usual way: code to keep in step, URLs to protect, and a slower page.',
    },
    () => {
      camera.focus(browser, { maxZoom: 1.4, room: { wide: { right: 300 }, tall: {} } })
      tl.wait(0.6)
      browser.highlightRow()
      stage.say(browser, {
        side: { wide: 'right', tall: 'below' },
        point: { wide: browser.at('.wt-rows', 'right'), tall: browser.at('.wt-rows', 'bottom') },
      })
      tl.wait(0.6)
    },
  )

  tl.beat(
    'Move the code',
    {
      caption: 'In Novis, the code of each API route moves into the page, unchanged. Core\\Html::later runs it.',
      notes: 'The same calls, Blog::latestPosts() and Shop::deals(). Only where they run changes.',
    },
    () => {
      browser.highlightRow(undefined, false)
      onEditor()
      // The usual run stays faded under the next one, on the same time axis.
      server.clear({ keep: 0.22 })
      browser.clearRows()
      browser.screen('blank')
      tl.wait(0.6)
      for (const [route, label, lines] of [
        [POSTS_ROUTE, 'Blog::latestPosts()', posts],
        [DEALS_ROUTE, 'Shop::deals()', deals],
      ] as const) {
        tl.wait(editor.file(route))
        editor.highlight(BODY)
        tl.wait(0.3)
        card.label(label)
        tl.wait(card.appear(editor.lines(BODY, BODY, 150)))
        editor.unhighlight()
        tl.wait(editor.file(NOVIS))
        editor.close(route)
        tl.wait(card.moveTo(editor.lines(lines[0], lines[1], 150)))
        card.leave()
        tl.wait(editor.insert(lines[0], lines[1], 80))
        tl.wait(0.2)
      }
      editor.highlight(posts[0], deals[1])
      stage.say(editor, {
        side: { wide: 'left', tall: 'below' },
        point: { wide: linesLeft(posts[0], deals[1]), tall: () => editor.edge('bottom') },
      })
      tl.wait(0.4)
    },
  )

  tl.beat(
    'One line',
    {
      caption: 'Delete the script. Then add one line: Core\\Response::slotted() sends the page before the slow parts are done.',
      notes: 'The last tab closes. The file is the whole change.',
    },
    () => {
      editor.unhighlight()
      tl.wait(editor.close(SCRIPT))
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
    'One request',
    {
      caption: 'The visitor opens the page again. The slow parts start at once, inside the same request.',
      notes: 'Both bars start right under the main code, far left of the faded ones.',
    },
    () => {
      editor.glow(0)
      editor.unhighlight()
      browser.unfill('posts')
      browser.unfill('deals')
      camera.focus('all', { only: 'wide' })
      camera.focus(browser, { only: 'tall' })
      tl.wait(0.7)
      const opened = open()
      tl.wait(0.15)
      run = { ...opened, ...parts((part) => part) }
      tl.wait(0.2)
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
        focus: [browser, server],
      })
      until(run.t0 + MAIN)
    },
  )

  tl.beat(
    'Page now',
    {
      caption: 'The page is here, and both slow parts are still running. You wrote no script for this.',
      notes: 'The moment. The page is there, and both bars are still growing. Let it land.',
      moment: true,
    },
    () => {
      tl.slow(0.5)
      run.main.phase('sends the parts', 'var(--wt-after)')
      server.marker(run.main, 'Browser has the page')
      camera.focus([browser, server], { only: 'wide' })
      camera.focus(browser, { only: 'tall' })
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
    'Sooner',
    {
      caption: 'Each part fills its place in the same response. Both are done sooner than in the usual way.',
      notes: 'Deals first, then posts. Compare each bar with the faded bar of the usual way.',
    },
    () => {
      tl.slow(1)
      camera.focus([browser, server], { only: 'wide' })
      camera.focus(server, { only: 'tall' })
      until(run.t0 + DEALS)
      run.deals.end()
      tl.wait(wire.pulse('back', 0.6, 'var(--wt-after)'))
      browser.fill('deals')
      until(run.t0 + POSTS)
      run.posts.end()
      tl.wait(wire.pulse('back', 0.6, 'var(--wt-after)'))
      browser.fill('posts')
      run.main.end()
      run.row.finish(200, 915)
      browser.loading(false)
      tl.wait(0.3)
      stage.say(server, {
        side: 'above',
        point: {
          wide: (t) => {
            const p = run.posts.tip(t)
            return { x: p.x + 20, y: p.y - 20 }
          },
          tall: () => server.edge('top'),
        },
        lift: 40,
        focus: [browser, server],
      })
      tl.wait(0.6)
    },
  )

  tl.beat(
    'One file',
    {
      caption: 'One file and one request. No script to write, and no API route to protect. It works in every modern browser.',
      notes: 'Point at the one row. The reference pages for Core\\Html::later and Core\\Response::slotted come next.',
    },
    () => {
      camera.focus(browser, { maxZoom: 1.4, room: { wide: { right: 300 }, tall: {} } })
      tl.wait(0.8)
      browser.highlightRow(0)
      stage.say(browser, {
        side: { wide: 'right', tall: 'below' },
        point: { wide: browser.at('.wt-row', 'right'), tall: browser.at('.wt-row', 'bottom') },
      })
      tl.wait(1.4)
    },
  )

  bindControls(stage)
  stage.start()
}
