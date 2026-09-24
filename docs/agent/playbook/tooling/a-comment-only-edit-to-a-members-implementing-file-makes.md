- **A comment-only edit to a member's implementing file makes every figure in that file stale, and
  the goal's own acceptance check then goes red on features the session never touched.**
  `rule:testing/member-perf-ledger` re-measures a figure when the implementing file's text moves
  with its `mod tests` cut off, so one `# Known gaps` paragraph added to `arr.rs` put nine green
  `Core\Arr` members back in the `perf` column of `python tools/dossier.py --group 'Core\Arr'
  --owed`. Close them with one `--record-perf` over the whole group after `nv verify` is green,
  rather than reading that column as new work. [until: reviewed 2026-09-20]
