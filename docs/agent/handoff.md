# Handoff

## State

**Stage 4 is on disk, `#[Api]` included.** `nvs build --openapi <file>` emits ADR 0085 § 3's document,
`nvs api diff` is § 4's gate, and `#[Api]` is § 2's annotation with all four contradictions refused.
`crates/nvs-cli/tests/openapi.rs` is 10 tests, all green, which is every test both stage-4 checks name.

**The stage-4 acceptance check was failing on a driver bug, not on the tree** — `tools/loop.py`'s
command-check branch inverted `ordered_in`, so no `kind = "command"` check has passed since `058c1f0`
introduced the kind. Fixed; the playbook bullet above owns the shape of that mistake. Four sessions
reported it as a stale binary, so a `## State` claim that an acceptance failure is the driver's is worth
one look at the branch that produced it.

**`#[Api]` is `crates/nvs-types/src/routes.rs`.** `API_OPTIONS` is four `OptionTy::Mixed` rows — that
module's own doc says why a roster shared with four other attributes is the wrong home for one
attribute's array-and-shape structure — and `check_api` reads the values with the declaration in hand.
Its doc comment owns the two readings this session had to settle:

- § 2's "not a class the handler could produce" is `ClassLinks::concrete`, **not** "declared". A name
  resolving to nothing is already `E0303` from the expression walk, so refusing it here named one
  mistake twice; what survives is the question `E0303` cannot ask — an interface, an `abstract` class or
  an enum is nothing a handler answers with. Declaredness is asked of `graph` **and** `signatures`
  because neither holds every kind: an enum has no graph entry, a member-less interface no signature row.
- § 2's `security` scheme check is written to the shape and no further. "A name no configured scheme
  defines" needs a configured scheme, and nothing in the tree declares one — `crates/nvs-cli/src/openapi.rs`'s
  gap list already records it. Refusing every name against an empty roster would refuse ADR 0085 § 2's
  own example.

**`Foo::class` is now an ADR 0046 § 2 constant** (`nvs_types::attributes::is_constant`), because § 2's
`errors` entry writes its `type` as a name and the only other spelling is a magic string, which ADR 0061
exists to refuse. It was already folded to the resolved name by `expr::members::check_class_name_const`,
so this admits a value the checker already had, not a new kind.

**Fixtures are one file per contradiction**, `crates/nvs-cli/tests/fixtures/api/`: each must be the only
refusal in its own build, plus `api-that-agrees.nvs` carrying all four fields correctly — four refusing
fixtures alone pass just as well against an `#[Api]` that refuses everything.

**Orientation gap, eighth session running:** `[context] adrs` still does not carry `0085 §§ 1-4`, and
this session sliced § 2 by hand.

## Next group

**What the return type and the doc comment let the document say. Shared file set:**
`crates/nvs-cli/src/openapi.rs`, `crates/nvs-types/src/routes.rs`, `crates/nvs-cli/tests/openapi.rs`.

- [ ] **Summary and description, § 1** — gap 3 in `crates/nvs-cli/src/openapi.rs`'s module doc, and the
      cheapest of the six: a handler's doc comment is already parsed, so this is carrying it to the row
      and splitting first-line-from-rest at the emitter. The row is `Route` at
      `crates/nvs-types/src/routes.rs:302`, built in `collect_route` at `:684`; the operation object is
      written in `crates/nvs-cli/src/openapi.rs`. One test in `crates/nvs-cli/tests/openapi.rs`.
- [ ] **Response schemas, § 1** — gap 1 in the same module doc. The handler's declared return type
      becomes the `200` response's schema, which `check_api_example` (`routes.rs:@check_api_example`)
      already resolves through `signatures.get(class).methods[name].return_ty` — read it the same way
      rather than a second walk, and reuse the property roster for the object schema.
- [ ] **`#[Api]`'s four fields reach the document, § 2.** Nothing emits `tags`, `errors`, `security` or
      `example` yet: they are checked and dropped. The row has no field for them, so this is one
      `ApiAnnotation` on `Route` filled in `check_api` and read in `openapi.rs`.

## Backlog

- A `.nvst` case for `#[Api]`'s refusals — `--EXPECTF-ERROR--`, as `#[Query]`/`#[Access]` got; the Rust
  tests cover all four today. `docs/agent/conventions.md` § *A `.nvst` test case*.
- A configured security-scheme roster, which § 2's second contradiction needs before it can be asked in
  full — `crates/nvs-cli/src/openapi.rs`'s gap list.
- `[context] adrs` in `docs/agent/loop-goal.toml` wants `0085 §§ 1-4`; eight sessions have now sliced it
  by hand.
- The remaining three gaps in `crates/nvs-cli/src/openapi.rs`'s module doc.
- `Core\Router` splits and `::match`, out of scope by the goal's own standing decisions.
