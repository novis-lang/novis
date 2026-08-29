# Handoff

## State

**`E0772` is on disk: a link refuses a literal `$params` value outside ADR 0102 § 5's closed set.** The
set is `RouteParam::allowed` (`crates/nvs-types/src/routes.rs`), computed by `closed_set` where the
interner is still alive, and the refusal is `nvs_types::links::within_set` — third in the short-circuit
after `covered` and `declared`, because a value is only a question once its key names something. A
`#[Query]` parameter's set is checked by the same walk, since ADR 0102 § 3 gives it the same type list.

**A *computed* out-of-set value is substituted and throws nothing**, and that decision's home is
`substitute`'s doc paragraph in `crates/nvs-stdlib/src/router.rs`: carrying the sets into a prepared
template would re-answer at run time what § 5 answers where the value is knowable, and § 5's own answer
to an out-of-set segment is a `404` rather than an error. The laundering is unchanged either way.

**The conformance corpus is at 912 and the driver's floor is 950**, so the failing acceptance check is 38
short and is still item 10's corpus floor rather than a regression. This session added three cases.

**A hole this group uncovered and did not take:** a parameter default at a literal or union declared type
is `E0451`, so § 3's *optional* `#[Query]` cannot be written at a closed set at all — see the playbook
bullet, and the backlog item with its anchor.

**Untouched:** item 12's roster (10 `UNCLASSIFIED` members at `crates/nvs-stdlib/src/registry.rs:1526`),
a `bytes` array key ICE in `nvs-ir`, and `catch (Core\Error $e)` panicking — the last two have playbook
bullets under *Writing a test case*.

**Orientation gaps.** Newly owed: `crates/nvs-cli/src/openapi.rs` in `[context] modules`, which the next
group's file set is. Still owed, none added yet: `crates/nvs-types/src/links.rs` and `src/routes.rs` in
`[context] modules`, ADR `0102 § 5` in `[context] adrs`, `docs/spec/02-php-migration.md` and
`tools/check-migration.py` in `[context] docs`, the stage-7 comment header's per-goal floor table, a
selector that prints the *failing* check's own `cases` block, a `[context] anchors` entry for
`registry.rs`'s `UNCLASSIFIED`, ADR 0088 § 1, 0063 R11, `0085 §§ 1-4`, `0071 §§ 2, 7`, and
conventions.md's *four shapes a depth case takes* whenever the group is conformance depth.

## Next group

**Shared file set:** `crates/nvs-cli/src/openapi.rs`, `crates/nvs-cli/tests/openapi.rs` and its fixtures
under `crates/nvs-cli/tests/fixtures/api`. All three slices are that module's *Known gaps* 5, which is
now answerable because the route row carries the set.

- [ ] **A closed-set parameter emits `enum:` rather than the empty schema** (`openapi.rs:199`
      `parameter`, `openapi.rs:221` `schema`, the gap at `openapi.rs:49`) — ADR 0102 § 5 names this
      explicitly ("so the generated document emits `enum: [en, de, fr]` with no further work") and ADR
      0085 § 1's *Enumerations* row is what it points at. `RouteParam::allowed` is the set;
      `schema(param.ty)` takes only the rendered type today, so `parameter` is what has to hand both
      over.
- [ ] **`Core\Uuid` emits `{"type": "string", "format": "uuid"}`** (`openapi.rs:221` `schema`) — the
      second name in the same gap sentence, and the last capture type with an unambiguous JSON Schema
      that renders as `{}` today.
- [ ] **The case for both** (`crates/nvs-cli/tests/openapi.rs`, fixtures under `tests/fixtures/api`) —
      the bound on both sides: a `"en"|"de"|"fr"` capture gets its three values, a `string` one still
      gets `{"type": "string"}` and no `enum` key.

## Backlog

- A parameter default at a literal or union declared type is `E0451` (`crates/nvs-types/src/defaults.rs:397`
  `literal_default`, the match at `:413`) — blocks § 3's optional closed-set `#[Query]`.
- An enum-case capture has no closed set to check (`crates/nvs-types/src/links.rs` *Known gaps* 2), and
  cannot have one until `Core\Router::match` decides a case's segment spelling.
- ADR 0097 § 3's mount prefix has nowhere to come from off the command line (`nvs-stdlib/src/router.rs`
  *Known gaps*).
- Item 12's roster: 10 `UNCLASSIFIED` members across five classes, `nvs-stdlib/src/registry.rs:1526`.
- The corpus floor: 912 of 950, `docs/agent/loop-goal.toml`'s stage-7 check.
