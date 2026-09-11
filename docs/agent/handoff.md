# Handoff

## State

**Goal `config-is-written` — stage 3's first check is green and its second is 3 of 4 tests in.**
A project command that resolves no tree now writes the shipped default file into the working
directory and reads that file back: `crates/nvs-cli/src/config.rs:126`'s `Init` is the gate,
`crates/nvs-cli/src/main.rs:214`'s `initializes` is the table of the five project commands that
produces it, and `crates/nvs-cli/src/config.rs:180`'s `write_default_file` is the write.

**`nvs_config::resolve::Roots` already reported which step won**, so `nvs-config` owed this slice
only `LOCAL_FILE` — one home for the name step 2 looks for and step 3 writes. `--no-init` is a
global flag and `NOVIS_NO_INIT` disables on presence; a named `--config` needs no gate of its own,
because step 1 is then the answer whether or not the files exist.

**Two things stage 3 still owes.** A declined write is silent — `write_default_file` answers
`Option<PathBuf>` and says nothing about why — so the boot line is unchanged, and there is still no
`nvs init`: `crates/nvs-cli/src/main.rs:193`'s `Command` has no `Init` variant.

**One open design consequence, and it is not a slice.** `nvs check` is in the writing table, so a
directory it has run in once holds a tree, and
`rule:config/no-configuration-file-is-a-complete-configuration` says a tree that was read and says
nothing about capabilities is an operator's written `no`. The *second* `nvs check` in a fresh
directory therefore refuses a literal `Core\Db::open` host that the first one passed.
`crates/nvs-cli/src/config.rs:296`'s `grants` handles only the run that did the writing — it asks
about the roots as they were **found**, so a file this very invocation manufactured is still no
tree, which is what keeps `crates/nvs-cli/tests/check_grants.rs:88` true. Run two is not covered and
cannot be without deciding which of two landed statements gives way: that rule's sentence, or
`nvs_config::default_file`'s claim that writing the file does not change the run that takes it.
Stage 4's record is where that belongs.

## Next group

**Stage 3: the last refusal and the declined record — one file set:**
`crates/nvs-cli/src/config.rs` and `crates/nvs-cli/src/main.rs`, with
`crates/nvs-cli/src/cache.rs:1480` read for its fixture.

- [ ] **`a_working_directory_that_fails_the_ownership_check_is_not_written_to`.**
      `crates/nvs-cli/src/config.rs:180` asks `nvs_config::trust::check` about the directory before
      it creates anything, per `rule:config/ownership-is-the-trust-boundary`; what is missing is the
      case. The fixture it needs is `crates/nvs-cli/src/cache.rs:1480`'s `open_to_the_world`, which
      is private to that module's `mod tests` — decide there whether to make that module
      `pub(crate)` or to lift the helper into one test-support module, and do it once.
- [ ] **The declined write says which reason applied, and the run stays green.**
      `a_read_only_working_directory_leaves_the_run_green_on_the_shipped_defaults` and
      `a_declined_write_says_which_reason_applied`. Widen
      `crates/nvs-cli/src/config.rs:180`'s return to carry the reason and emit the one `Info` record
      the goal's stage 3 § 5 and § 7 ask for, surfaced through
      `crates/nvs-cli/src/config.rs:252`'s `boot_in`. `rule:config/the-resolved-root-is-announced-and-stored`
      is the boot line this joins, and it is `designed` rather than shipped — check whether that
      announcement exists at all before adding a second place that prints one.
- [ ] **`nvs init` writes the same file and refuses to overwrite one.**
      A new `Command::Init` beside `crates/nvs-cli/src/main.rs:193`'s other arms, calling
      `write_default_file` and reporting what every refusal above points an operator at.
      `rule:config/the-root-is-config-else-nvs-toml-else-the-shipped-defaults` step 3 is what it
      writes; it is not a project command and does not go through the `initializes` table.

## Backlog

- Stage 4's record owes the second-`nvs check`-run consequence in § *State* a decision, not a
  paragraph — `docs/agent/loop-goal.md` § *Stage 4*.
- `nvs build` is in the `initializes` table and resolves no tree, so it writes nothing today —
  `crates/nvs-cli/src/main.rs:214`.
- The pack printed only § *Standing decisions* from `docs/agent/loop-goal.md`; the stage's own
  numbered prose is what specifies the write, and `[context]` has no selector for it.
- Carried gaps that outlive this goal: `docs/agent/carried-gaps.md`.
