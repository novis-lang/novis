# Handoff

## State

Milestone `dossier`, goal `config-directives-1-3` — 16 features, **9 landed**, 7 open. Each of
`directive:debug.keep_temporary`, `directive:debug.inline` and `directive:deferred.deadline` now has
its Rust census test in `crates/nvs-config/tests/directives.rs`, one example with a blessed `.out`,
an attack and an `about.md`; `python tools/dossier.py --group 'config:directives'` is the only
scoreboard worth reading. A directive owes **one** example and no bench —
`tools/data/dossier-policy.toml`'s table is what each kind owes.

`nvs.toml` gained two blocks. `[debug] keep_temporary = false` is what an absent `[debug]` block
already meant, written so the page has a value to print. The `[[app]]` block beside the two page
blocks is the first written for an *attack*: the keep_temporary case has to create the directories
it hoards before the sweep has anything to come apart on, and the grant is `fs` unscoped because
`Core\IO::temporaryDir` creates under the host's own temporary directory. Nothing is blocked.

## Next group

**Goal `config-directives-1-3`, items 10–12 — one file set:** `crates/nvs-config/tests/directives.rs`
(`keys_in` lists a block's accepted keys, `keys_listed` takes a whole block header for a shape
`[block]` cannot spell, and `governing` resolves a row), `nvs.toml`,
`docs/examples/config/<dir>/` and `tests/hostile/config/<dir>/`. The directory name keeps a key's
underscores and turns only its dots into dashes, so ask `python tools/dossier.py --id '<feature>'`
rather than deriving it.

- [ ] **`directive:deferred.max_concurrent`** — `System` and `Reload`, the half of `[deferred]` that
      sizes the host: it counts request *trees* per core rather than registrations, and what it
      spends is `max_concurrent × [limits.hard]` memory on top of the in-flight requests,
      `rule:concurrency/deferred-is-bounded-by-two-directives`.
      `crates/nvs-config/src/directive.rs:242`. The pairing is already asserted from the deadline's
      side at `crates/nvs-config/tests/directives.rs:774`, so this row's census wants the cost and
      the `afterResponse` throw past the cap, not the split again.
- [ ] **`directive:extension`** — `System` and `Reload`,
      `rule:packaging/extension-loading-is-root-controlled`.
      `crates/nvs-config/src/directive.rs:247`, the one row in that run with no comment above it.
      `[[extension]]` is an array of tables, so `keys_in("extension")` writes the wrong shape and
      lists nothing; `keys_listed("[[extension]]\nnvs_no_such_key = true\n")` is the spelling
      (`crates/nvs-config/tests/directives.rs:191`), and `nvs_config::tree::Extension` is the two
      keys it should answer.
- [ ] **`directive:http`** — `Runtime` and `Reload`,
      `crates/nvs-config/src/directive.rs:121`. The census test already exists as
      `every_http_response_directive_is_runtime_class`
      (`crates/nvs-config/tests/directives.rs:845`) and owes only a `// covers: directive:http`
      marker above it, so the work here is the example, the attack and `about.md`.

## Backlog

- `Core\Config::get` answers nothing for every list-valued key, so a scoped grant and an absent one
  are one answer from inside a program — designed, but said nowhere a `Core\Config` reader looks
  (`crates/nvs-stdlib/src/config.rs`).
- The end-of-script sweep leaves a handed-out path that is no longer a directory
  (`crates/nvs-runtime/src/sweep.rs:178`), and the orphan walk skips non-directories on purpose
  (`:251`), so a program that swaps its own temporary directory for a file plants one entry under
  the owned root that nothing ever reclaims.
  `tests/hostile/config/debug-keep_temporary/01-hoarding-what-the-sweep-must-delete.nvs` § 7 is the
  demonstration, and the sweep's own doc argues the under-deletion — overturning it is a decision
  record rather than a slice.
- A `Runtime` directive outside `[limits]` has no ceiling to compare against
  (`crates/nvs-config/src/request.rs:286-297`), so `Core\Config::set('deferred.deadline', 'banana')`
  is accepted and read back verbatim. The `Ok(None)` arm at `:130-135` says that is deliberate; what
  no doc says is that the overlay then holds a value nothing can parse.
- The goal's remaining seven features, `directive:deferred.max_concurrent` onwards — `python
  tools/dossier.py --group 'config:directives'`.
