/**
 * What the front page's numbers show. The counts are src/data/numbers.json,
 * which `bun nv render` writes from the repository; what each one counts is
 * written in tools/nv/renderers/website-numbers.ts. The measured figures are
 * Novis's in one case of the recorded benchmark run that src/lib/compare.ts
 * reads, and are `null` before a run is recorded.
 */
import data from '../data/numbers.json'
import { CASE_TEXT, record } from './compare'

/** The counts, each rounded down to its first two digits except the two crate counts. */
export const counts = data as {
  attacks: number
  languageTests: number
  rustTests: number
  examples: number
  benchmarks: number
  editorTests: number
  crates: number
  cratesWithoutUnsafe: number
}

/** A rounded-down count as text: `at least 1,400`. */
export const atLeast = (n: number) => `at least ${n.toLocaleString('en-US')}`

/** The case the front page shows. */
const CASE = 'json-api'

export interface Measured {
  /** What the case's program does, as the comparison page says it. */
  caseText: string
  medianRequestsPerSec: number
  p99Ms: number
  peakMiB: number
  concurrency: number
  cpus: number
  processor: string
  /** `11 October 2026`. */
  date: string
  reps: number
  /** Every case together. */
  cases: number
  requests: number
  failed: number
}

function measure(): Measured | null {
  const c = record?.cases[CASE]
  const nvs = c?.peers.nvs
  if (!record || !c || !nvs || nvs.p99_ms === null || nvs.memory_peak_bytes === null) return null
  const cases = Object.values(record.cases)
  const requests = cases.length * record.requests * record.reps
  const ok = cases.reduce((n, k) => n + (k.peers.nvs?.status_codes['200'] ?? 0), 0)
  return {
    caseText: CASE_TEXT[CASE]!.text,
    medianRequestsPerSec: Math.round(nvs.median_requests_per_sec),
    p99Ms: nvs.p99_ms,
    peakMiB: nvs.memory_peak_bytes / 1048576,
    concurrency: c.concurrency,
    cpus: record.backend_cpus,
    processor: (record.host.processor ?? '').replace(/\s+\d+-Core Processor\s*$/, '').trim(),
    date: new Date(record.date).toLocaleDateString('en-GB', { day: 'numeric', month: 'long', year: 'numeric' }),
    reps: record.reps,
    cases: cases.length,
    requests,
    failed: requests - ok,
  }
}

export const measured: Measured | null = measure()
