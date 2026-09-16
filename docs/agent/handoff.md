# Handoff

## State

Goal `m8-stdlib-depth`. **Stages 0 and 2–8 are done, and stage 9's two acceptance checks are both
green**: the throw producer landed last session, and `rule:errors/renderings`' third rendering is on
disk now. All three renderings exist in `nvs-render`; what no code does yet is *select* the HTML one,
which is stage 9 item 2's tail and the next group. Nothing is blocked.

`crates/nvs-render/src/html.rs` holds the record rendering beside the sink `escape` it writes every
piece of model text through — the module doc owns the collapsible/typed/class-aware shape and why
every `<details>` is `open`. A `Node::Cycle` is an `<a href>` to the object's anchor. `crate::hex`
(`crates/nvs-render/src/lib.rs:542`) is now the one hex all three renderings write, and that crate's
`# Known gaps` block renumbered when the HTML gap was struck: gap 1 is the `#[Test]` producer, gap 2
the compiler diagnostic.

## Next group

**Stage 9, item 2's tail: the HTML rendering is what a request in force selects** — one file set:
`crates/nvs-stdlib/src/debug.rs`, `crates/nvs-runtime/src/ctx/output.rs`,
`crates/nvs-config/src/tree.rs` and `crates/nvs-config/src/mode.rs`.

- [ ] **`Core\Debug::dump` picks its rendering from the sink in force, and never from an argument** —
      `crates/nvs-stdlib/src/debug.rs:156` calls `nvs_render::plain::render_nodes` unconditionally.
      `OutputSink::Body` (`crates/nvs-runtime/src/ctx/output.rs:89`) is the row
      `rule:errors/renderings` gives `Core\Html\Markup`, and `nvs_render::html::render_nodes`
      (`crates/nvs-render/src/html.rs:168`) is what a dump writes there. `Core\Debug::render`
      (`crates/nvs-stdlib/src/debug.rs:170`) already answers the sink's carrier and takes the same
      split; `crates/nvs-stdlib/src/debug.rs:247`'s `rendered` stays plaintext, because
      `rule:testing/inline-snapshots`' snapshot is that text.
- [ ] **`[debug] inline` is a directive with a field, a row and a template line** —
      `crates/nvs-config/src/mode.rs:71` already derives it per mode, while
      `crates/nvs-config/src/tree.rs:421`'s `Debug` block carries only `mode` and `keep_temporary`,
      so nothing reads the key. `rule:testing/debug-mode-directive` is the rule; `verify.py`'s
      `directives` and `template` steps both gate the row and the default file, and they fail
      separately from the build.

## Backlog

- The `[debug] inline` **block** — the collapsible markup appended to an HTML response body with its
  style under the CSP nonce — is `docs/decisions/0092.md:423-425`. It needs `response.rs`, which this
  goal's § *Not this goal* gives to goal `m7-server-surface`; the rendering it would append now
  exists.
- The `#[Test]` result producer is `crates/nvs-render/src/lib.rs:39`'s gap 1. Its code is
  `crates/nvs-cli/src/runner.rs` and `crates/nvs-stdlib/src/test.rs`, and `test.rs` is on this goal's
  § *Not this goal* list as goal `m7-server-surface`'s — which the chain has already walked. Stage 9's
  prose and acceptance ask for neither; the gap's owner line still says this goal.
- `crates/nvs-runtime/src/floor.rs:133`'s `report_argument` skips a non-scalar field, so a throw's
  `properties` do not reach the tier-3 handler's array — deliberate, and stated there.
- The plaintext rendering is still uncoloured — `crates/nvs-render/src/plain.rs:15`, waiting on
  nothing now that `Core\Cli` exists.
