# Handoff

## State

Goal `core-http-stream` is reached. Every member of `Core\Http\Stream` (`chunks`, `events`, `header`,
`headers`, `lines`, `saveTo`, `status`) has every feature proof, and `python tools/dossier.py --id
'Core\Http\Stream'` owes nothing. `Core\Http\Stream`, `Core\Http\Chunks`, `Core\Http\Lines` and
`Core\Http\Events` all carry their class cards (`crates/nvs-stdlib/src/http/stream.rs`), and the
last two are struck from `CLASSES_STILL_OWING_A_CARD`. `saveTo`'s programs write into
`Core\IO::temporaryDir()` under their own `fs` blocks in the root `nvs.toml`. The perf ledger holds
a current figure for all ten `Core\Http\Stream` and `Core\Http\Event` members. No proof of this
class found a bug in it.

## Next group

**Stage 1: whatever the next goal's handoff names** — the driver switches goals and overwrites this
file, so this session names no group of its own.

- [x] **`Core\Http\Stream::saveTo` and the `Lines`/`Events` cards** — landed. `rule:testing/feature-proofs`. `crates/nvs-stdlib/src/http/stream.rs:248`

## Backlog

- `Core\Json::decode` and `Core\Json::decodeAs<T>` refuse any `tainted string` with `E0401`, even
  where `T`'s text fields are declared `tainted`. ADR 0071's M4S line says that call compiles.
  `crates/nvs-types/src/expr/quals.rs`'s `admits_tainted_argument` refuses a `Contagious` parameter
  whose return type cannot carry the qualifier, and `crates/nvs-types/src/derive.rs`'s
  `reads_a_peers_octets` says the refusal is deliberate. So a line from `Core\Http\Stream::lines()`
  cannot be decoded as JSON Lines, although the member's own card names JSON Lines. A design
  question for the user, not a proof slice.
- `Core\Http\Stream::saveTo` takes the body before it asks the `fs.write` door, so a refused save
  leaves a stream no other reader can read. The Rust test pins this as it is; whether a refusal
  should leave the body readable is a question for `rule:core-classes/io-write-stream`'s owner.
