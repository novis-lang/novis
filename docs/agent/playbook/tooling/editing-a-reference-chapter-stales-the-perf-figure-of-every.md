- **Editing a reference chapter stales the perf figure of every feature in it.** A language feature's
  implementing file *is* its `docs/reference/lang/*.md` chapter, so `rule:testing/member-perf-ledger`
  re-measures all eighteen of `lang:types` when one paragraph moves, and `dossier.py --verify
  --group` then reports `perf: stale` against features nobody touched. Re-record the group in one
  call: `python tools/dossier.py --record-perf --group lang:types`, straight after any chapter edit
  rather than after reading that failure. [until: gone tools/dossier.py:impl_hash]
