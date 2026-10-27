/**
 * What the comparison page under In-Depth reads: src/data/compare.json,
 * which `bun nv render` copies from benches/compare.json, the record that
 * `bun nv bench-compare --record` writes. Its shape is `buildRecord`'s in
 * tools/nv/cmd/bench-compare.ts. Before a run is recorded the file has
 * `recorded: false`, and each part of the page says that no run is recorded.
 *
 * The text a reader sees for each case, row and result is here, beside the
 * ids the record uses, so a case or row the record gains without text here
 * fails the build.
 */
import data from '../data/compare.json'
import { githubFile } from '@config/site.mjs'

interface PeerFigures {
  /** The median of the run's repetitions. */
  requests_per_sec: number
  /** The same figure as `requests_per_sec`. */
  median_requests_per_sec: number
  /** From the median repetition. */
  p99_ms: number | null
  memory_peak_bytes: number | null
  /** How many responses had each status, over every repetition. */
  status_codes: Record<string, number>
  /** The server settings the case ran with, as the run recorded them. */
  settings?: Record<string, unknown>
}

type Verdict = 'built-in' | 'on-by-default' | 'one-setting' | 'your-code' | 'no'

interface RowResult {
  result: Verdict
  setting?: string
  /** What the row measured, in the shape its judge in bench-compare.ts writes. */
  evidence?: Record<string, unknown>
}

interface CompareRecord {
  recorded: true
  /** When the run was recorded, local time with no zone: `2026-10-11T14:34:18`. */
  date: string
  backend_cpus: number
  requests: number
  reps: number
  host: { platform?: string; processor?: string; cpus?: number }
  peers: Record<string, { label: string }>
  cases: Record<string, { concurrency: number; peers: Record<string, PeerFigures> }>
  scenarios: { rows: Record<string, { folder: string; peers: Record<string, RowResult> }> } | null
}

const raw = data as { recorded: false } | CompareRecord

/** The recorded run, or `null` before the first one. */
export const record: CompareRecord | null = raw.recorded ? raw : null

export const CASE_TEXT: Record<string, { title: string; text: string }> = {
  hello: { title: 'Hello', text: 'Returns a short text for every request.' },
  'json-api': { title: 'JSON API', text: 'Reads an order as JSON, checks it, and returns the order with its totals as JSON.' },
  'html-page': { title: 'HTML page', text: 'Builds a page of 100 products and escapes a search term in it.' },
  wait: { title: 'Waiting', text: 'Waits 50 ms on a timer for each request, with 1000 connections open.' },
}

const ROW_TEXT: Record<string, string> = {
  cores: 'Many requests at once are spread over four CPU cores',
  crash: 'Other requests are still answered after one request crashes',
  loop: 'A request that loops forever is stopped, and other requests are still answered',
  memory: 'A request that uses too much memory is stopped, and other requests are still answered',
  flood: 'With more load than it can handle, the server still answers other requests in time, and recovers when the load stops',
  escape: 'Text in an HTML page is always escaped, also when you write it without a template',
  taint: 'An SQL injection from outside data (request, session, environment, database) is not executed',
  secret: 'A secret in the output is found before the program runs',
  csrf: 'A form sent from another site is rejected',
  session: 'A request that only reads the session does not wait for a slow request of the same visitor',
  headers: 'Security headers are sent by default',
  waiting: 'Answers 1000 waiting requests on one CPU',
  parallel: 'One request can split its work over four CPU cores',
  'live-edit': 'An edited source file is served without a restart',
  deploy: 'A new version goes live under load with no failed request and no mixed reply',
  'deploy-big': 'A big deploy of 2,000 files goes live under load with no failed request and no mixed reply',
  'deploy-switch': 'Switching a "current" link to a new release goes live with no failed request and no mixed reply',
  config: 'A change to the config file applies without a restart',

}

/**
 * The heading on the comparison page that explains a row in more detail, by its id there. A row with
 * no entry has no explanation beyond its own line.
 */
const ROW_DETAILS: Record<string, string> = {
  escape: 'escaped-text',
  taint: 'outside-data',
  'live-edit': 'edited-files',
  deploy: 'deploying-a-new-version',
  'deploy-big': 'deploying-a-new-version',
  'deploy-switch': 'deploying-a-new-version',
  cores: 'many-requests-at-once',
  loop: 'runaway-requests',
  memory: 'runaway-requests',
  session: 'one-visitor-two-requests',
  waiting: 'many-waiting-requests',
  parallel: 'one-request-in-four-parts',
}

const VERDICT_TEXT: Record<Verdict, string> = {
  'built-in': 'Built in',
  'on-by-default': 'On by default',
  'one-setting': 'One setting',
  'your-code': 'Your code',
  no: 'No',
}

function textOf<T>(table: Record<string, T>, id: string, what: string): T {
  const found = table[id]
  if (found === undefined) throw new Error(`compare: the record has a ${what} "${id}" that src/lib/compare.ts has no text for`)
  return found
}

/**
 * How a peer takes a new version. A `files` peer serves a new source file once it is copied in, and a
 * `build` peer needs a restart, and Go a new build too. Only the deploy chart is split by it, because
 * the deploy row is the one test that changes files under load, and the two ways pay for a new version
 * differently. Every other chart sends one unchanged program, so it ranks all four peers together.
 */
export type Group = 'files' | 'build'

const GROUPS: Group[] = ['files', 'build']

const GROUP_TEXT: Record<Group, { title: string; note: string }> = {
  files: {
    title: 'Live deploy',
    note: 'Novis and PHP use new source files as soon as they are copied to the server. There is no build, no restart and no reload.',
  },
  build: {
    title: 'Build and restart',
    note: 'Bun and Go load new code only when the server restarts. Go also needs a new build first.',
  },
}

const PEER_GROUP: Record<string, Group> = { nvs: 'files', php: 'files', bun: 'build', go: 'build' }

/** The peer this website is about, whose bars every chart marks. */
export const HOME_PEER = 'nvs'

export interface Peer {
  id: string
  label: string
  group: Group
}

/** The peers in the record's order. */
export const peers: Peer[] = record
  ? Object.entries(record.peers).map(([id, p]) => ({ id, label: p.label, group: textOf(PEER_GROUP, id, 'peer') }))
  : []

const groupOf = (peer: string): Group => textOf(PEER_GROUP, peer, 'peer')

/** `items` split by their peer's group, in `GROUPS` order, leaving out a group with no item. */
export function grouped<T extends { peer: string }>(items: T[]): { group: Group; title: string; note: string; items: T[] }[] {
  return GROUPS.flatMap((group) => {
    const inGroup = items.filter((t) => groupOf(t.peer) === group)
    return inGroup.length === 0 ? [] : [{ group, ...GROUP_TEXT[group], items: inGroup }]
  })
}

/**
 * How close two figures are when the page counts them as the same result: the run's margin of error,
 * which the comparison page states as about 3%.
 */
export const MARGIN = 0.03

/**
 * Each figure's place in its test, 1 for the best, in the order of `values`. A figure within `MARGIN`
 * of the first figure of a place shares that place, and the next place counts the shared ones:
 * 1, 1, 3. The first figure, not the last one added, is the measure, so a run of close figures
 * cannot carry a place further than the margin.
 */
export function places(values: number[], lessIsBetter: boolean): number[] {
  const order = bestFirst(values, lessIsBetter)
  const place = new Array<number>(values.length)
  let first = 0
  let firstPlace = 0
  order.forEach((i, rank) => {
    const v = values[i]!
    if (rank === 0 || Math.abs(v - first) > Math.abs(first) * MARGIN) {
      first = v
      firstPlace = rank + 1
    }
    place[i] = firstPlace
  })
  return place
}

/** The indexes of `values`, best figure first. Equal figures keep their order. */
function bestFirst(values: number[], lessIsBetter: boolean): number[] {
  return values.map((_, i) => i).sort((a, b) => (lessIsBetter ? values[a]! - values[b]! : values[b]! - values[a]!))
}

/**
 * Each figure's position when the figures are sorted best first, 0 for the best, in the order of
 * `values`. A chart draws the figures that share a place in this order, so the better figure of a tie
 * is drawn first.
 */
export function positions(values: number[], lessIsBetter: boolean): number[] {
  const position = new Array<number>(values.length)
  bestFirst(values, lessIsBetter).forEach((i, rank) => (position[i] = rank))
  return position
}

/** `places` counted inside each peer's group, where `peerIds[i]` is the peer of `values[i]`. */
export function groupPlaces(peerIds: string[], values: number[], lessIsBetter: boolean): number[] {
  const place = new Array<number>(values.length)
  for (const group of GROUPS) {
    const at = peerIds.flatMap((id, i) => (groupOf(id) === group ? [i] : []))
    const inGroup = places(at.map((i) => values[i]!), lessIsBetter)
    at.forEach((i, k) => (place[i] = inGroup[k]!))
  }
  return place
}

export interface Bar {
  peer: string
  label: string
  requestsPerSec: number
  /** This peer's requests per second over the best peer's in the case, from 0 to 1. */
  share: number
  /** The peer's place among all peers by requests per second, from `places`. */
  place: number
  /** The peer's position by requests per second, from `positions`. */
  position: number
  best: boolean
  p99Ms: number | null
  memoryMiB: number | null
  /** The peer's place by peak memory among the peers with a figure, or `null` with no figure. */
  memoryPlace: number | null
  /**
   * The case ran PHP with `opcache.validate_timestamps` off, so a copied file was not live. The speed
   * cases turn it off because PHP is faster without it, and the page says so beside PHP's group.
   */
  fileCheckOff: boolean
}

export interface CaseChart {
  id: string
  title: string
  text: string
  concurrency: number
  bars: Bar[]
}

/**
 * Every recorded case, each peer's throughput against the best peer's in that case. Places are counted
 * over all four peers, because a case sends one unchanged program, and every peer in place 1 is `best`,
 * so the accent colour and the place agree.
 */
export const charts: CaseChart[] = record
  ? Object.entries(record.cases).map(([id, c]) => {
      const best = Math.max(...Object.values(c.peers).map((p) => p.requests_per_sec))
      const figures = peers.flatMap((p) => (c.peers[p.id] === undefined ? [] : [{ peer: p.id, label: p.label, f: c.peers[p.id]! }]))
      const speeds = figures.map(({ f }) => f.requests_per_sec)
      const speedPlaces = places(speeds, false)
      const speedPositions = positions(speeds, false)
      const withMemory = figures.filter(({ f }) => f.memory_peak_bytes !== null)
      const memoryPlaces = places(
        withMemory.map(({ f }) => f.memory_peak_bytes!),
        true,
      )
      const bars = figures.map(({ peer, label, f }, i) => ({
        peer,
        label,
        requestsPerSec: f.requests_per_sec,
        share: best > 0 ? f.requests_per_sec / best : 0,
        place: speedPlaces[i]!,
        position: speedPositions[i]!,
        best: speedPlaces[i] === 1,
        p99Ms: f.p99_ms,
        memoryMiB: f.memory_peak_bytes === null ? null : f.memory_peak_bytes / 1048576,
        memoryPlace: f.memory_peak_bytes === null ? null : memoryPlaces[withMemory.findIndex((m) => m.peer === peer)]!,
        fileCheckOff: f.settings?.['opcache.validate_timestamps'] === '0',
      }))
      return { id, ...textOf(CASE_TEXT, id, 'case'), concurrency: c.concurrency, bars }
    })
  : []

export interface Cell {
  verdict: Verdict
  text: string
  setting: string | null
}

export interface FeatureRow {
  id: string
  text: string
  /** The folder on GitHub with the four programs this row runs. */
  url: string
  /** The page's own link to the heading that explains this row, or `null` for a row with none. */
  details: string | null
  /** One cell per peer in `peers` order, or `null` for a peer the row has no result for. */
  cells: (Cell | null)[]
}

/** The feature table, one row per recorded scenario, in the record's order. */
export const rows: FeatureRow[] = record?.scenarios
  ? Object.entries(record.scenarios.rows).map(([id, r]) => ({
      id,
      text: textOf(ROW_TEXT, id, 'row'),
      url: githubFile(r.folder),
      details: ROW_DETAILS[id] === undefined ? null : `#${ROW_DETAILS[id]}`,
      cells: peers.map((p) => {
        const c = r.peers[p.id]
        return c ? { verdict: c.result, text: textOf(VERDICT_TEXT, c.result, 'result'), setting: c.setting ?? null } : null
      }),
    }))
  : []

/**
 * How a figure where less is better compares with the least one: `best` for
 * the least itself, else how much more it is, as `89% slower` with
 * `more` = `slower`. From twice the least up it is a factor, `37× more`,
 * which reads more easily than `3600% more`.
 */
export function overLeast(value: number, least: number, best: string, more: string): string {
  if (value <= least) return best
  const ratio = value / least
  return ratio >= 2 ? `${ratio.toFixed(ratio < 10 ? 1 : 0)}× ${more}` : `${Math.round((ratio - 1) * 100)}% ${more}`
}

/** The evidence a row recorded for each peer, in `peers` order, skipping a peer the row has no result for. */
function evidenceOf(row: string): { peer: string; label: string; evidence: Record<string, unknown> }[] {
  const r = record?.scenarios?.rows[row]
  if (r === undefined) return []
  return peers.flatMap((p) => {
    const evidence = r.peers[p.id]?.evidence
    return evidence === undefined ? [] : [{ peer: p.id, label: p.label, evidence }]
  })
}

/** The rows that deploy a new version under load: 200 files, 2,000 files, and a switched `current` link. */
export type DeployRow = 'deploy' | 'deploy-big' | 'deploy-switch'

/** What a deploy row's load saw while a new version was copied in, summed over the row's runs. */
export interface Deploy {
  peer: string
  label: string
  /** Every reply the load checked while the new version went in. */
  requests: number
  failed: number
  /** Replies built partly from the old files and partly from the new ones. */
  mixed: number
}

export function deploysOf(row: DeployRow): Deploy[] {
  return evidenceOf(row).map(({ peer, label, evidence: e }) => ({
    peer,
    label,
    requests: e.requests as number,
    failed: e.failed_requests as number,
    mixed: e.mixed_replies as number,
  }))
}

/** What the steady load of the `memory` or `loop` row saw while one runaway request ran beside it. */
export interface Runaway {
  peer: string
  label: string
  /** The requests answered while the load ran, which lasts the same time for every peer. */
  requests: number
  /** The requests the same load answered on the same server with no runaway request. */
  baseline: number
  /** `requests` over `baseline`: the share of its own speed the server kept, from 0 to 1. */
  kept: number
  failed: number
  p99Ms: number | null
  /** The server was running after the test. */
  running: boolean
  /** How often the server crashed and its restart policy started it again. */
  restarts: number
  /** The seconds until the server stopped the runaway request itself, or `null` when it never did. */
  stoppedAfter: number | null
}

/** nginx's own answers for a backend that did not reply: such a reply is not the server stopping the request. */
const PROXY_STATUSES = new Set([502, 504])

export function runaways(row: 'memory' | 'loop'): Runaway[] {
  return evidenceOf(row).map(({ peer, label, evidence: e }) => {
    const requests = e.requests as number
    const baseline = e.baseline_requests as number
    const status = (e.runaway_status as number | null) ?? null
    const seconds = (e.runaway_seconds as number | null) ?? null
    return {
      peer,
      label,
      requests,
      baseline,
      kept: baseline > 0 ? Math.min(1, requests / baseline) : 0,
      failed: e.failed_requests as number,
      p99Ms: (e.p99_ms as number | null) ?? null,
      running: e.backend_running as boolean,
      restarts: (e.backend_restarts as number | undefined) ?? 0,
      stoppedAfter: status === null || PROXY_STATUSES.has(status) ? null : seconds,
    }
  })
}

/** A peer's figure with one core and with four, as the `cores` and `parallel` rows measured it. */
export interface OneAndFour {
  peer: string
  label: string
  /** Requests per second with one core for `cores`, the seconds one part took for `parallel`. */
  one: number
  /** Requests per second with four cores for `cores`, the seconds four parts at once took for `parallel`. */
  four: number
  /** How the program runs its parts at once, for `parallel`. */
  way: string | null
}

/** How long the `session` row's fast request waited, for each peer that ran it. */
export interface SessionWait {
  peer: string
  label: string
  /** The milliseconds from sending the fast request to its reply. */
  waitMs: number
  /** The code that releases the session early, where the peer needs it. */
  code: string | null
}

/** The `session` row: the fast request's wait for each peer that has sessions, Novis and PHP. */
export const sessionWaits: SessionWait[] = evidenceOf('session')
  .filter(({ evidence: e }) => typeof e.fast_wait_ms === 'number')
  .map(({ peer, label, evidence: e }) => ({
    peer,
    label,
    waitMs: e.fast_wait_ms as number,
    code: (e.code as string | null) ?? null,
  }))

/** The `cores` row: the throughput of a one-part `parallel` request with one core and with four, for every peer. */
export const manyRequests: OneAndFour[] = evidenceOf('cores').map(({ peer, label, evidence: e }) => ({
  peer,
  label,
  one: e.requests_per_sec_1_cpu as number,
  four: e.requests_per_sec_4_cpus as number,
  way: null,
}))

/** The `parallel` row: one request's work as one part and as four parts, for each peer that has a way to run parts at once. */
export const oneRequest: OneAndFour[] = evidenceOf('parallel')
  .filter(({ evidence: e }) => e.way !== null)
  .map(({ peer, label, evidence: e }) => ({
    peer,
    label,
    one: e.seconds_1_part as number,
    four: e.seconds_4_parts as number,
    way: e.way as string,
  }))
