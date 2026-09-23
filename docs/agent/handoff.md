# Handoff

## State

Goal `core-http-stream` is under way: `Core\Http\Stream::chunks`, `::events` and `::header` have
every feature proof, and `Core\Http\Stream` and `Core\Http\Chunks` carry their class cards
(`crates/nvs-stdlib/src/http/stream.rs`). Four members of the class still owe proofs: `status`,
`headers`, `lines` and `saveTo`. `Core\Http\Lines` and `Core\Http\Events` still owe a card in
`CLASSES_STILL_OWING_A_CARD` (`crates/nvs-stdlib/src/registry.rs`); `dossier.py` does not ask for
them, only for the class a member belongs to. No proof found a bug.

## Next group

**Stage 1: the rest of `Core\Http\Stream`** — one file set: `crates/nvs-stdlib/src/http/stream.rs`'s
`mod tests`, `tests/conformance/core/http-stream-reads-lines-and-chunks-and-refuses-a-second-read.nvst`
and the `Http-Stream` trees under `docs/examples/`, `tests/hostile/` and `benches/members/`. The
three landed members are the shape: a Rust test builds a `STREAM` with `crate::instance::build` and a
bare `Value::uint` key, and needs no reader.

- [ ] **`Core\Http\Stream::lines`** — owes examples, hostile, perf, a Rust test; `rule:testing/feature-proofs`. `crates/nvs-stdlib/src/http/stream.rs:229`
- [ ] **`Core\Http\Stream::headers`** — owes both tests (a `covers:` line on the `.nvst` above gives the Novis one), examples, hostile, perf. `crates/nvs-stdlib/src/http/stream.rs:211`
- [ ] **`Core\Http\Stream::status`** — the same as `headers`. `crates/nvs-stdlib/src/http/stream.rs:193`
- [ ] **`Core\Http\Stream::saveTo`** — owes a Rust test, examples, hostile, perf; its examples need an `nvs.toml` granting `fs.write`. `crates/nvs-stdlib/src/http/stream.rs:247`

## Backlog

- `Core\Http\Lines` and `Core\Http\Events` have no class card — goal `core-class-cards` (`crates/nvs-stdlib/src/registry.rs`).
- Known gap 1 in `crates/nvs-stdlib/src/http/stream.rs`'s module doc (the copied `id`) — owner M12.
