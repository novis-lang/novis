# Handoff

## State

**Goal 6, M7. ADR 0074 § 2's open half is half-landed: a *simple* request from a named origin now
crosses.** `nvs_server::cors::Cors::answer` reads the request's `Origin` against the list resolved
from `[http.cors] origins` and returns a `Crossing`, which the door writes onto the answer beside
`Secure`'s set (`crates/nvs-server/src/serve.rs:627`, `:727`). An origin is matched **exactly** —
no port, case or trailing-slash equivalence — and `Vary: Origin` goes on *every* answer an open
named list touched, including the refused origin's and the request that sent none; the wildcard is
the one open policy that does not vary. That module's doc owns the whole argument, including why
the two pre-handler refusals deliberately carry none of it.

**The driver's failing check is an open item, not a regression.** `nvs-server (match once, and the
two answers)` names four ADR 0102 tests. There is no route table anywhere in the tree —
`crates/nvs-stdlib/src/router.rs`'s own module doc says `Core\Router::match` is unwritten — so it
is a milestone-sized item, and two of its four names cannot be hosted by `-p nvs-server` at all
(`Core\Request::route()` is `nvs-stdlib`'s, which `crates/nvs-server/Cargo.toml` does not name in
either dependency table). The playbook bullet added this session is the triage.

`orient.py` printed ADR 0074 § 5 but not § 2, which is the section this goal's CORS slices are
specified by — `[context] adrs` should name `0074:2`.

## Next group

**The rest of ADR 0074 § 2 — what a *preflight* from a named origin is answered with.** Both slices
are the same two files as the landed one: `crates/nvs-server/src/cors.rs` and
`crates/nvs-server/src/serve.rs`. `nvs_config::tree::HttpCors`
(`crates/nvs-config/src/tree.rs:411`) is where `methods`, `headers`, `expose`, `credentials` and
`max_age` are already deserialized and waiting; nothing in `nvs-server` reads them yet.

- [ ] **A preflight from a named origin is answered with what § 2 configures** (ADR 0074 § 2).
      `Cors::preflight` currently stands aside whenever the list is non-empty
      (`crates/nvs-server/src/cors.rs:140`); it grows into the answer — `Allow-Methods`,
      `Allow-Headers` and `Max-Age` from the block, `204`, and the same `Vary` rule `answer`
      already applies. `crates/nvs-server/src/cors.rs:154` is `Cors::answer`, whose `Crossing` the
      preflight answer should reuse rather than duplicate, and
      `crates/nvs-server/src/serve.rs:613` is the door's preflight branch.
- [ ] **`expose` and `credentials` reach a simple response** (ADR 0074 § 2).
      `Access-Control-Expose-Headers` and `Access-Control-Allow-Credentials` are two more fields on
      `Crossing` (`crates/nvs-server/src/cors.rs:102`), filled by `Cors::answer`
      (`crates/nvs-server/src/cors.rs:154`) and written by `Crossing::fill`
      (`crates/nvs-server/src/cors.rs:197`). `nvs_config::http` already refuses `["*"]` beside
      `credentials = true` at boot, so that pair is never this crate's question.

## Backlog

- ADR 0102's route table — the driver's failing check, and a milestone-sized item with no
  foundation on disk yet (`docs/adr/0102-…md` § 1, `crates/nvs-stdlib/src/router.rs`).
- `[context] adrs` in `docs/agent/loop-goal.toml` is missing `0074:2`.
- `Core\Session` and its storage backend — ADR 0012 § 4 defers the mechanics to M7/M8.
- ADR 0105's open upload names, per the split check in `loop-goal.toml` stage 5.
