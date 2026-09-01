# Handoff

## State

**ADR 0126 is lowered end to end: `$obj->$key` reads and writes the field it names.** § 5's choice is
recorded in the ADR body and in `nvs-ir`'s known gap 21 — one instruction per direction rather than a
closed-set chain over the roster. `crates/nvs-types/src/expr/members.rs`'s `check_keyed_property` now
records `ExprInfo::KeyedProperty { class, ty }` at the access span, carrying § 5's union;
`crates/nvs-ir/src/lower/expr.rs` and `lower/stmt.rs` lower it to `InstKind::KeyGet`/`KeySet`, which
`nvs-codegen` hands to the new `nvs_runtime::nvs_object_key_get`/`::nvs_object_key_set` — ADR 0036 § 4's
erased access with the name arriving as a `Value` by address instead of as a static byte range. The
erased *read* is now `read_erased_property_hinted`, factored out of `nvs_object_slot_get` so the two
callers share one lookup, mirroring what `write_erased_property` already was.

**`lower_property_access`'s catch-all is a consistency claim again**, not a lowering still owed:
`check_property_member`'s doc comment holds its proof rather than owing it, and the entry is recorded even
where `T` declares no public property, since "unreachable at run time" is not "never lowered".

**The stage-8 acceptance check is closed**: all three `.nvst` cases it names are written and green.

Owed on item 35: § 5's `E0782` refusal for a write through a key whose set holds a `readonly` property,
and the empty-set half of `E0799` (siting still undecided — the playbook bullet a previous session left
says why the obvious home cannot work). The keyed access inherits ADR 0036 § 4's own gap, hooks bypassed
on both directions; that is recorded on `nvs_object_key_get` and closes for every caller at once.

## Next group

**§ 5's two remaining refusals plus the erased path's hook gap, over
`crates/nvs-types/src/expr/assign.rs`, `crates/nvs-types/src/expr/members.rs` and
`crates/nvs-runtime/src/object.rs`.**

- [ ] **`E0782` through a key** — ADR 0126 § 5's last paragraph: a write through a `property<T>` whose
      public set holds a `readonly` property is refused at the write, naming it. The code and the
      headline are already `crates/nvs-types/src/expr/assign.rs:648`'s
      `E_READONLY_WRITE_AFTER_CONSTRUCTION`; what is new is that the check must run over the *set*
      rather than over one resolved property, and it has to know the access is a write —
      `check_keyed_property` at `crates/nvs-types/src/expr/members.rs:594` is only told `is_unset`
      today, so the write flag threads the same way `access_span` just did
      (`crates/nvs-types/src/expr/members.rs:868`, `crates/nvs-types/src/expr/mod.rs:428`). The § 5
      console block is the exact wording.
- [ ] **The empty-set half of `E0799`** — ADR 0126 § 1 refuses a written `property<T>` whose `T`
      declares no public property at all. `check_keyed_property` already answers `mixed` and records
      an entry for that case (`crates/nvs-types/src/expr/members.rs:640`), so the refusal is owed at
      the *annotation* rather than here; the E07xx band is FULL at `E0799`, which is what the previous
      session's playbook bullet says makes the obvious home unworkable. Decide the siting and record
      it in § 1.
- [ ] **The hooked-property gap on the erased path** — ADR 0036 § 4's own, at
      `crates/nvs-runtime/src/object.rs:2510` (the read) and
      `crates/nvs-runtime/src/object.rs:2586` (the write): both reach the slot past a per-property
      hook (ADR 0014 § 1). Now that three callers share them — § 4's own access,
      `Core\Reflect\ClassInfo::get`/`set` and ADR 0126's key — closing it once closes it everywhere.
      `ClassDesc` carries no hook table today, which is the cost to weigh.

## Backlog

- Stage 7: reading `[log] target` — `docs/plan/m8.md`.
- Stage 2: `Core\IO\File::truncate` and `::lock` — `docs/spec/01-core-library.md` § 13.
- Stage 3: `Core\Cli::displayWidth` — ADR 0086 § 1.
- ADR 0014 § 3's `onPropertyGet` never fires on an erased or keyed read — decide whether that is a gap
  or the rule, in ADR 0036 § 4.
