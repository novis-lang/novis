# Handoff

## State

**Goal 6, M7. The door reads its own match for two of ADR 0102 § 1's three rules.**
`nvs_server::route::csrf_required` is ADR 0096 § 4's coverage question and `::label` is
ADR 0076 § 1's `route` label; each is a field of the row `take` already found, and the test
drops the table before asking, so an answer that survives cannot have been re-derived. § 8's
access decision has no reader there on purpose — it is the dispatcher's. **The driver's
failing acceptance check named exactly that test and it now runs.**

**§ 1a's `csrf: false` crosses on the row to get there.** `nvs_types::routes::Route::csrf` is
the declaration's half and `nvs_runtime::routes::Route::new` derives § 4's verb half, which is
what makes a table built by hand fail closed; `without_csrf` is the rare case.

**What the door still does not do is refuse, and that is a seam rather than an omission.** § 4's
refusal needs the token verified, and `nvs-server` reaches neither half of ADR 0060's
verification: `Core\Csrf::verify` is `nvs-stdlib`'s, a crate above it, and no `[http]` directive
names a key. `crates/nvs-server/src/route.rs`'s module doc is the home of that gap and of the
label's — nothing exports a metric yet.

**`nvs_runtime::commands`' gap 1 is now one, not two: the enum.** A union of literal types
converts as `ArgConv::OneOf`, off the same `nvs_types::routes::closed_set` the route table's
captures use, and a word outside the set is a *usage* error naming every value that would have
been accepted. The value is the word, as `routes` answers the same set with `Param::Text`.

**The group's order was swapped, and the reason is in the code:** `closed_set` answers `None` for
an enum, so the union half needed only the `OneOf` row both halves want, where the enum half
needs two things nothing has decided — a case's written spelling, and the class named on the row
to build a value from. That is the next item, and the gap case still has the enum to use.

## Next group

**The enum command argument, then the corpus over both conversions.** The file set is the one
this session had open: `crates/nvs-types/src/commands.rs`, `crates/nvs-runtime/src/commands.rs`,
`crates/nvs-cli/src/main.rs` and `crates/nvs-stdlib/src/command.rs`.

- [ ] **`ArgConv` carries an enum's class and cases, and an enum argument converts**
      (ADR 0086 § 6) — the variant to add is beside `crates/nvs-runtime/src/commands.rs:72`'s
      `OneOf`, and it needs a *class* as well as a set, which `OneOf` deliberately does not
      carry. Two decisions this slice makes and records in that variant's own doc: the word a
      case is written as on a command line (the case name, or ADR 0010 § 3's backing value —
      `crates/nvs-types/src/routes.rs:1779`'s `closed_set` returns `None` rather than guess it),
      and the value it becomes, which wants the case and so wants
      `crates/nvs-runtime/src/object.rs:637`'s `EnumCases` off the class descriptor. The choice
      is `crates/nvs-types/src/commands.rs:196`, its admission is
      `crates/nvs-types/src/commands.rs:710`, the crossing is `crates/nvs-cli/src/main.rs:805`
      and the arm that turns text into a value is `crates/nvs-stdlib/src/command.rs:543`.
- [ ] **The corpus follows both conversions** (ADR 0086 § 6) — a `.nvst` case for a union
      argument admitted and refused, over `crates/nvs-stdlib/src/command.rs:543`'s arm and the
      usage text it produces; and the gap case, which still has the enum as a type no matcher
      converts. `crates/nvs-runtime/src/commands.rs:41`'s known gap 1 is what it pins.

## Backlog

- The CSRF *refusal*: needs a key at the door and a session to bind to — a configuration
  decision. `crates/nvs-server/src/route.rs` known gap 1.
- ADR 0076's exporter: nothing in the tree emits a series, so `route::label` has no caller.
  Same file, known gap 2.
- An `int` literal union binds its digits rather than the number, in both tables.
  `nvs_runtime::commands::ArgConv::OneOf`'s doc.
- `orient.py` did not print ADR 0086 § 6, and the previous handoff said so too; `[context] adrs`
  still wants `0086:6`. It did print 0096 § 4 and 0102 §§ 1/8, which is what this session needed.
