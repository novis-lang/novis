# Handoff

## State

Goal `core-http-socket-and-1-more` (dossier) is reached: every `Core\Http\Socket` member and all
three `Core\Http\Event` readers — `data`, `name`, `id` — report `complete.` under `python
tools/dossier.py --id`. `Core\Http\Event` now carries its class card and is off
`CLASSES_STILL_OWING_A_CARD`. The three readers are credited by `covers:` markers on
`http-stream-reads-sse-events-across-every-line-ending.nvst` and on the Rust test
`each_reader_answers_its_own_field_of_the_framed_event`. Their benches measure zero allocations.
`target/release/nvs.exe` was rebuilt this session. The `id` attack found that an `id` in force is
copied into every later event; that is `crates/nvs-stdlib/src/http/stream.rs`'s `# Known gaps` 1,
owner M12.

## Next group

**The driver switches to the next goal in the chain** — no file set is open here.

- [x] **`Core\Http\Event::data`, `::name`, `::id`** — `rule:testing/feature-proofs`; `crates/nvs-stdlib/src/http/stream.rs:397`.

## Backlog

- Share the `lastId` slot's one reference with every event whose block set no `id` — `crates/nvs-stdlib/src/http/stream.rs` `# Known gaps` 1; a new refcount edge, so it owes a valgrind run.
