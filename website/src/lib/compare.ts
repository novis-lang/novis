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
  requests_per_sec: number
  p99_ms: number | null
  memory_peak_bytes: number | null
}

type Verdict = 'built-in' | 'one-setting' | 'your-code' | 'no'

interface RowResult {
  result: Verdict
  setting?: string
}

interface CompareRecord {
  recorded: true
  date: string
  commit: string
  arm: string
  backend_cpus: number
  nginx_cpus: number
  requests: number
  reps: number
  host: { platform?: string; processor?: string; cpus?: number; docker?: string }
  peers: Record<string, { label: string }>
  cases: Record<string, { concurrency: number; peers: Record<string, PeerFigures> }>
  scenarios: { date: string; commit: string; rows: Record<string, { folder: string; peers: Record<string, RowResult> }> } | null
}

const raw = data as { recorded: false } | CompareRecord

/** The recorded run, or `null` before the first one. */
export const record: CompareRecord | null = raw.recorded ? raw : null

const CASE_TEXT: Record<string, { title: string; text: string }> = {
  hello: { title: 'Hello', text: 'Returns a short text for every request.' },
  'json-api': { title: 'JSON API', text: 'Reads an order as JSON, checks it, and returns the order with its totals as JSON.' },
  'html-page': { title: 'HTML page', text: 'Builds a page of 100 products and escapes a search term in it.' },
  wait: { title: 'Waiting', text: 'Waits 50 ms on a timer for each request, with 1000 connections open.' },
}

const ROW_TEXT: Record<string, string> = {
  cores: 'Uses more than one CPU core',
  isolation: 'A request cannot see a value that another request set',
  runaway: 'A request that loops or uses too much memory is stopped',
  crash: 'The server keeps running after a request crashes',
  escape: 'Text in an HTML page is escaped by default',
  taint: 'A request value in an SQL string is found before the program runs',
  secret: 'A secret in the output is found before the program runs',
  csrf: 'A form sent from another site is rejected',
  headers: 'Security headers are sent by default',
  waiting: 'Answers 1000 waiting requests on one CPU',
  parallel: 'One request can run its work on four cores',
  'live-edit': 'An edited file is served without a restart',
  deploy: 'A new version goes live with no failed request',
  config: 'A change to the config file applies without a restart',
  upgrade: 'A new runtime version runs with no failed request',
}

const VERDICT_TEXT: Record<Verdict, string> = {
  'built-in': 'Built in',
  'one-setting': 'One setting',
  'your-code': 'Your code',
  no: 'No',
}

function textOf<T>(table: Record<string, T>, id: string, what: string): T {
  const found = table[id]
  if (found === undefined) throw new Error(`compare: the record has a ${what} "${id}" that src/lib/compare.ts has no text for`)
  return found
}

export interface Peer {
  id: string
  label: string
}

/** The peers in the record's order. */
export const peers: Peer[] = record ? Object.entries(record.peers).map(([id, p]) => ({ id, label: p.label })) : []

export interface Bar {
  label: string
  requestsPerSec: number
  /** This peer's requests per second over the best peer's in the case, from 0 to 1. */
  share: number
  best: boolean
  p99Ms: number | null
  memoryMiB: number | null
}

export interface CaseChart {
  id: string
  title: string
  text: string
  concurrency: number
  bars: Bar[]
}

/** Every recorded case, each peer's throughput against the best peer's in that case. */
export const charts: CaseChart[] = record
  ? Object.entries(record.cases).map(([id, c]) => {
      const best = Math.max(...Object.values(c.peers).map((p) => p.requests_per_sec))
      const bars = peers
        .filter((p) => c.peers[p.id] !== undefined)
        .map((p) => {
          const f = c.peers[p.id]!
          return {
            label: p.label,
            requestsPerSec: f.requests_per_sec,
            share: best > 0 ? f.requests_per_sec / best : 0,
            best: f.requests_per_sec === best,
            p99Ms: f.p99_ms,
            memoryMiB: f.memory_peak_bytes === null ? null : f.memory_peak_bytes / 1048576,
          }
        })
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
  /** One cell per peer in `peers` order, or `null` for a peer the row has no result for. */
  cells: (Cell | null)[]
}

/** The feature table, one row per recorded scenario, in the record's order. */
export const rows: FeatureRow[] = record?.scenarios
  ? Object.entries(record.scenarios.rows).map(([id, r]) => ({
      id,
      text: textOf(ROW_TEXT, id, 'row'),
      url: githubFile(r.folder),
      cells: peers.map((p) => {
        const c = r.peers[p.id]
        return c ? { verdict: c.result, text: textOf(VERDICT_TEXT, c.result, 'result'), setting: c.setting ?? null } : null
      }),
    }))
  : []

/** `2026-10-10T08:00:00` as `2026-10-10`. */
export const day = (stamp: string) => stamp.slice(0, 10)
