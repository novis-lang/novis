# Handoff

## State

Goal `m8-stdlib-depth`. **Stages 0 and 2–8 are done; stage 9 is half landed.** Item 1 of stage 9 — the
throw producer — is on disk and both of its acceptance tests pass; item 2, the HTML rendering, is the
next group. Nothing is blocked.

`crates/nvs-runtime/src/record.rs` is now the **one** value-to-node walk, reached by both producers
that have a value to show: `Core\Debug::dump` calls it (`crates/nvs-stdlib/src/debug.rs`'s gap 3 is
struck, and its gaps 1 and 2 moved with the walk) and so does the floor, which may depend on no `Core`
class. An uncaught throw's record carries **one `Node::Frame` per frame in `record.nodes`** and no
`backtrace` field at all; the throw's own declared properties past `Throwable`'s four are one
`properties` field, walked, so a `secret` anywhere under them is `Node::Redacted`. The tier-3 handler's
report array still gets a `backtrace`, built from those nodes. `rule:errors/record-producers` was
amended in the same slice to name the Frame node and say why it is not an Object one.

Eight `.nvst` cases took the new JSON shape (`"fields":{"class":…},"nodes":[{"function":…}]`). The
floor/`Core\Log` schema case is one of them and is green.

## Next group

**Stage 9, item 2: the HTML rendering, the third of `rule:errors/renderings`' three** — one file set:
`crates/nvs-render/src/html.rs`, `crates/nvs-render/src/lib.rs`, `crates/nvs-render/src/plain.rs` and
`crates/nvs-render/src/json.rs` as the two to mirror.

- [ ] **A record renders as HTML, agreeing with plain and JSON on an elision and on a redaction** —
      `crates/nvs-render/src/html.rs:1` holds only the sink's `escape` today;
      `crates/nvs-render/src/json.rs:215` and `crates/nvs-render/src/plain.rs:117` are the two
      exhaustive `Node` matches to mirror, and `Node::Frame` (`crates/nvs-render/src/lib.rs:401`) is
      the newest kind. `rule:errors/renderings` is the rule and fixes the carrier as
      `Core\Html\Markup`. The named tests the acceptance asks for are
      `the_html_rendering_elides_where_plain_and_json_elide` and
      `a_secret_renders_as_the_placeholder_in_all_three_renderings`, in `nvs-render`.
- [ ] **`crates/nvs-render/src/lib.rs:35`'s gap 1 is struck and § *What is here* rewritten whole** —
      the `# Known gaps` block renumbers to the `#[Test]` producer and the compiler diagnostic, which
      the goal's § *Standing decisions* owns; `[debug] inline`'s wiring into a response is ADR 0092's
      M7 bullet and goal `m7-server-surface`'s unless that goal has already built it.

## Backlog

- The `#[Test]` result producer is `crates/nvs-render/src/lib.rs:41`'s gap 2, tagged M4S — which the
  goal's § *Standing decisions* reads as "built here, in stage 9". It is not in the stage's prose or
  its acceptance checks; decide it when item 2 lands.
- `crates/nvs-runtime/src/floor.rs:133`'s `report_argument` skips a non-scalar field, so a throw's
  `properties` do not reach the tier-3 handler's array — deliberate, and stated there.
- The plaintext rendering is still uncoloured — `crates/nvs-render/src/plain.rs:15`, waiting on
  nothing now that `Core\Cli` exists.
