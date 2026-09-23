# Handoff

## State

Goal `core-http-stream` is under way: `Core\Http\Stream::chunks`, `::events`, `::header`, `::lines`,
`::status` and `::headers` have every feature proof, and `Core\Http\Stream` and `Core\Http\Chunks`
carry their class cards (`crates/nvs-stdlib/src/http/stream.rs`). One member of the class still owes
proofs: `saveTo`. `Core\Http\Lines` and `Core\Http\Events` still owe a card in
`CLASSES_STILL_OWING_A_CARD` (`crates/nvs-stdlib/src/registry.rs`); `dossier.py` does not ask for
them, only for the class a member belongs to. No proof of this class found a bug in it; one example
found an ADR promise the type checker does not keep (Backlog).

## Next group

**Stage 1: the rest of `Core\Http\Stream`** — one file set: `crates/nvs-stdlib/src/http/stream.rs`'s
`mod tests` and the `Http-Stream` trees under `docs/examples/`, `tests/hostile/` and
`benches/members/`. The six landed members are the shape: a Rust test builds a `STREAM` with
`crate::instance::build` and a bare `Value::uint` key, and needs no reader.

- [ ] **`Core\Http\Stream::saveTo`** — owes a Rust test, examples, hostile, perf; its examples need an `nvs.toml` granting `fs.write`, the way `tests/conformance/core/http-stream-caps-a-line-and-one-events-data.nvst`'s `--FILE nvs.toml--` does. `rule:testing/feature-proofs`. `crates/nvs-stdlib/src/http/stream.rs:248`
- [ ] **Class cards for `Core\Http\Lines` and `Core\Http\Events`** — strike both from the list and write the cards the way `Core\Http\Chunks`'s is written. `rule:core-api/reference-card`. `crates/nvs-stdlib/src/registry.rs:5189`

## Backlog

- `Core\Json::decode` and `Core\Json::decodeAs<T>` refuse any `tainted string` with `E0401`, even
  where `T`'s text fields are declared `tainted`. ADR 0071's M4S line says that call compiles.
  `crates/nvs-types/src/expr/quals.rs`'s `admits_tainted_argument` refuses a `Contagious` parameter
  whose return type (`mixed`, a written `T`) cannot carry the qualifier, and
  `crates/nvs-types/src/derive.rs`'s `reads_a_peers_octets` says the refusal is deliberate. So a
  line from `Core\Http\Stream::lines()` cannot be decoded as JSON Lines, although the member's own
  card names JSON Lines. A design question for the user, not a proof slice.
