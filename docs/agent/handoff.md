# Handoff

## State

**Goal 6, M7. ADR 0102 § 1's first half is on disk: a request is matched once, at the door,
against the table its own unit declared.** `nvs_runtime::routes` is that table as a running
program carries it — the compiler's rows crossed as strings plus `CaptureConv`, § 2's four
segment forms, § 5's closed sets, and `matchit`'s left-to-right precedence stated as a rank.
Its module doc owns the whole argument, including the four known gaps (a linear scan rather
than ADR 0077 § 2's trie; `decimal`/`Core\Uuid` unconverted; no program-visible reader yet; a
capture's value still percent-encoded).

**The two halves sit where the ADR puts them.** The *table* is `Ctx::set_routes`, installed
before the program runs exactly as the command table is; the *match* is `Inbound::set_route`,
because it is a fact about the request and § 1's rule is that it is taken once and travels.
`nvs_server::route::take` is the door that takes it, and `nvs-cli`'s handler calls it where the
selected unit and the arrived request are first both in hand — `crate::script::Compiled` is why
the cache can answer with a table at all.

**Nothing in a program can see it yet**: `Core\Request::route()` and `Core\Router\Match` do not
exist (`nvs_runtime::routes` gap 3, and `nvs_stdlib::router` gap 3 from the other side). The
driver's `nvs-server (match once, and the two answers)` check therefore still fails, and this is
the ordinary open-item state: one of its four names now exists and passes, and the three below
are the rest.

## Next group

**ADR 0102's match reaching a program, and § 2's other answer.** The file set is
`crates/nvs-stdlib/src/request.rs`, `crates/nvs-stdlib/src/router.rs` and
`crates/nvs-runtime/src/routes.rs`, against the carrier at `crates/nvs-runtime/src/ctx.rs:4704`.

- [ ] **`Core\Router\Match` is a `Core` instance class, and `Core\Request::route()` answers one**
      (ADR 0102 § 1). The five edits go in `crates/nvs-stdlib/src/request.rs:192`'s `CLASS` and
      beside `crates/nvs-stdlib/src/router.rs:204`; what it reads is
      `crates/nvs-runtime/src/ctx.rs:4704`'s `Inbound::route`, whose `Match` already answers
      `name()`, `params()` and `param()` at `crates/nvs-runtime/src/routes.rs:377`. `null` where
      nothing matched is the carrier's `None` and needs no second case.
- [ ] **§ 2's two answers: `404` where a path has no methods, `405` with `Allow` where it has
      some** (ADR 0102 § 2). `methods_for` belongs beside the walk at
      `crates/nvs-runtime/src/routes.rs:442`, asked only after `match_request` answered `None`,
      and `{name?}` reports both forms. `Core\Router::methodsFor` is the program-side half, in
      `crates/nvs-stdlib/src/router.rs:204`'s `CLASS`.
- [ ] **The CSRF check and the `route` label read the match rather than matching again** (ADR
      0102 § 1, ADR 0096 § 4, ADR 0076 § 1). Both read `crates/nvs-server/src/route.rs:57`'s
      answer off the carrier; the door's enforcement half is what does not exist yet, so read
      ADR 0096 § 4 before assuming the test is only an assertion.

## Backlog

- ADR 0077 § 2's trie replaces the linear scan — `crates/nvs-runtime/src/routes.rs` gap 1.
- `decimal` and `Core\Uuid` captures convert — same module, gap 2, and `commands`' gap 1 is the twin.
- A capture's percent-decoding, once `nvs_stdlib::uri`'s decoder is the one home — gap 4.
- ADR 0102 § 7's mount captures reaching a handler as `tainted` values — the check's fourth name.
- Raw/unparsed body access for an arbitrary content-type — ADR 0024 *Revisiting*, `docs/plan/m7.md`.
- ADR 0105 §§ 3-4's `readAll`/`content`/`saveTo` and `Core\IO::writeStream`.
