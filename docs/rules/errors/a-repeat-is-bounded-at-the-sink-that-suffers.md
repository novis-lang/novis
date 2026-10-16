A run of identical records is bounded at the sink that would suffer from it, and each sink answers
for what is scarce in it: a disk-bounded target coalesces before the write, and an indexed target
stores every occurrence and groups them at read time.

`rule:http-server/the-floor-cannot-fill-the-disk` puts the bound on the sink rather than on the
caller "so that no caller has to be trusted to be rare", and that holds here unchanged. What differs
is what each sink is protecting. The diagnostic log's bound is a finite disk, and the cheapest place
to protect one is before the bytes exist. The debug stream's bound is an index that holds a million
rows without complaint, so merging occurrences before storing them would spend the developer's
information to save nothing scarce — its ingester already computes a per-record hash, and the viewer
collapses a run into one row saying how many it stands for, which expands.

**The log target's window is a small fixed table, not the floor's single slot.** The floor keeps one
because a fault loop repeats one record; application code interleaves, and a single slot coalesces
none of that.

The identity is the floor's: `ts`, `request_id`, `trace_id`, `span_id` and any existing `count` are
cleared before hashing, because those are what distinguish two occurrences of one thing. Everything
else counts, `source` included — two identical messages from two lines are two facts. **A record that
differs is written immediately and never held behind a window**, and the next occurrence after a
window closes carries how many it stands for.

Coalescing bounds what is *stored*, not what is *spent*: a duplicate is only known to be one after
its record has been built and rendered, so a loop still pays for every record it makes.

- **Memory:** each worker thread holds one fixed table of `LOG_WINDOW_SLOTS` windows
  (`crates/nvs-runtime/src/floor.rs`), whatever the program writes. A record below `[log] level`
  takes no slot.

**Declined:**
- **One coalescing policy for every sink.** The floor, a disk-bounded log and an index protect
  different things, and one policy would either fill a disk or discard what a developer went looking
  for.
- **A per-call-site rate limit.** It is the only option that makes a hot loop cheap rather than
  merely quiet, but it drops records that differ, which is a change of meaning rather than an
  optimisation. Nothing measured asks for it. Taking it would need a shed record to increment an
  exported counter, as `rule:errors/log-fields` already requires.
