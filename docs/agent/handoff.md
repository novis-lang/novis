# Handoff

## State

**Goal 6, M7. ADR 0102 § 7's mount is half landed, at the door.** `nvs_runtime::Inbound` carries the
prefix ADR 0097 § 4 step 2 stripped and § 3's glob captures of the row that stripped it;
`nvs_server::mount::carry` writes them, mirroring `route::take`, and `nvs-cli`'s handler calls it
where it already calls `set_peer`. The acceptance check that named this was one name over two
claims and is now two checks — the door's half is green, and `tainted` moved to the crate that
declares it, `nvs-stdlib`.

**What § 7 still owes is `Core\Request::mount()`, and its open question is the return type.** § 7
spells `{prefix: string, captures: array<tainted string>}`, and a shape is not spellable in return
position: `CoreTy::Shape` is a parameter-only variant whose own doc says "no runtime representation
of a shape appears anywhere", and `nvs_ir::lower_checked_ty` erases a `Ty::Shape` value to
`Ty::Object`. So the member either gains a return-position shape or is spelled as an instance class
the way `Core\Router\Match` already is (`crates/nvs-stdlib/src/router.rs:322`) — the second is the
cheap one and costs an edit to § 7's body, which the goal's § *Standing decisions* pre-authorizes.

**An enum command argument converts, and `nvs_runtime::commands`' gap 1 is now only a case
*subset*.** `nvs_types::commands::ArgConv::Enum` carries the class and every case as a
`(word, value)` pair, and both decisions are recorded in its own doc: the word is the **case name**
(§ 6's refusal names every accepted value because a command line is a person typing, and a list of
backing integers is not that sentence), and the value is the case's backing integer unwidened, so
nothing is constructed and no class descriptor is reached at run time.
`nvs_types::routes::closed_set` still answers `None` for an enum and that stays — it serves ADR 0102
§ 5's captures too, whose spelling is out of scope.

## Next group

**§ 7's member, over one file set:** `crates/nvs-stdlib/src/request.rs`,
`crates/nvs-stdlib/src/router.rs` and
`docs/adr/0102-a-request-is-matched-once-and-the-route-table-completes-without-dispatching.md`.

- [ ] **`Core\Request::mount()` answers § 7's two facts** (ADR 0102 § 7) — take the return-type
      decision first and record it in the row's own doc; `Core\Router\Match` is the worked shape at
      `crates/nvs-stdlib/src/router.rs:776` and its name constant at
      `crates/nvs-stdlib/src/router.rs:322`. The row goes beside `route`'s at
      `crates/nvs-stdlib/src/request.rs:297` and the body beside its helper at
      `crates/nvs-stdlib/src/request.rs:1500`; what it reads is
      `crates/nvs-runtime/src/ctx.rs:4756`'s `mount_captures` and the `mount_prefix` above it. The
      captures are `tainted` — `crates/nvs-stdlib/src/request.rs:319`'s `HEADER_LINES` is the
      spelling for a nested `CoreTy::TaintedStr`. An instance class means editing § 7's body to say
      so, folded, in the same commit.
- [ ] **The `-p nvs-stdlib` test the split check names** (ADR 0102 § 7) —
      `core_request_mount_answers_the_prefix_and_its_captures_as_tainted_strings`, in that module's
      own test block at `crates/nvs-stdlib/src/request.rs:2633`: it asserts the registry row's declared
      qualifier, which is the half `nvs-server` could not host, plus the member reading back what
      `Inbound::set_mount` wrote. Three `.nvst` cases are owed with the member
      (`conformance_coverage.rs`'s floor), and `Core\Request` outside a request throws, so one of
      them is that refusal.
- [ ] **The corpus follows the union conversion** (ADR 0086 § 6) — the enum half landed with
      `tests/conformance/core/command-run-converts-an-enum-argument-by-its-case-name.nvst` and its
      refusal sibling; `ArgConv::OneOf` still has no `.nvst` case, and the shape to copy is those
      two. `crates/nvs-types/src/commands.rs:184` is the variant.

## Backlog

- A subset of an enum's cases is still `ArgConv::Unconverted` — `crates/nvs-runtime/src/commands.rs`'s
  gap 1, now narrowed to that and stating what closing it needs.
- § 4's CSRF refusal needs a verified token and a `[http]` key — `crates/nvs-server/src/route.rs`'s
  known gap 1.
- Nothing exports ADR 0076's `route` label — `crates/nvs-server/src/route.rs`'s known gap 2.
- Raw body access for an arbitrary content-type — ADR 0024's *Revisiting*, narrowed by `docs/plan/m7.md`.
