# Handoff

## State

Goal `lang-programs`, eight of its nine features finished with all four proofs plus `about.md`:
`require-run-another-file-in-this-frame` and `autoload-find-a-class-by-its-namespace` landed this
session. `python tools/dossier.py --verify --group lang:programs` names one owing,
`ending-a-program`, and that is the whole of the next group. Both new benches declare
`// bench: calls 1` and `// bench: allocations 0` and both measure them; the figures are in
`docs/perf/members.ndjson`, which also carries a re-measure of the group's other seven features,
because the chapter fix below moved the `impl_hash` every one of them is keyed on.

`docs/reference/lang/10-programs.md` § *`require`* said a required file runs "in the caller's own
frame — same variables". It does not, and the binary agrees with the rule rather than the chapter:
`rule:statements/a-required-file-shares-declarations-not-locals` is the home and says declarations
cross and locals do not. The lead sentence and a new bullet now say what the rule says.

Both `require` and `autoload` need a second file beside the example program, and
`docs/examples/README.md` § *What an example is* now says where it goes: a subdirectory of the
example's own directory, because the sweep counts `*.nvs` at the top level only and the website
mirror walks the whole tree. The three proof trees all follow it.

Two findings are in `## Backlog` rather than in code. Both need a call only the user can make, so
neither is a `# Known gaps` item: `python tools/owners.py --check` takes a milestone tag or nothing,
and no milestone at M9 or later scopes either one.

## Next group

**Stage 2: the dossier, the group's last feature and the goal's end** — one file set:
`docs/reference/lang/10-programs.md`, a fresh directory each under `docs/examples/lang/programs/`
and `tests/hostile/lang/programs/`, `tools/data/dossier-policy.toml`, and two `.nvst` cases already
on disk that need only a `covers:` line. Shapes are `docs/examples/README.md`,
`tests/hostile/README.md` and `rule:testing/four-proofs`.

- [ ] **`lang:programs/ending-a-program`** — owes about, 3 examples, 2 tests, hostile. `exit;`,
      `exit(3);` and `exit("message");` are the three forms, `die` does not exist, and an uncaught
      throw ends the program with status 1. `docs/reference/lang/10-programs.md:254`
- [ ] **Its perf is a `[skip]`, not a bench** — a program can exit once, so there is no loop to
      measure, which is the case the goal's § *Standing decisions* names by hand ("a member whose
      program exits"). Write `[skip."lang:programs/ending-a-program"]` with `perf = "<one
      sentence>"`; the section is empty today and this is the tree's first entry, so the shape is
      the one `load_policy` reads. Say so in the commit. `tools/data/dossier-policy.toml:44`
- [ ] **Then the goal's end gates, then `DONE`** — `python tools/verify.py --doc`, then
      `python tools/owners.py --closes lang-programs` and `python tools/playbook.py --closes
      lang-programs`, fixing what each names before the status line goes in.
      `docs/agent/loop-goal.toml:12212`

## Backlog

- A computed `require` path runs nothing and hands back `1`, silently — `require 'lib/' . $name .
  '.nvs';` is a no-op. `crates/nvs-ir/src/lower/stmt.rs:403` and `lower/expr.rs:507` emit nothing
  when `require_target` has no entry, and `crates/nvs-hir/src/requires.rs:12` leaves a non-literal
  path for a "runtime fallback" that exists nowhere in the tree.
  `docs/reference/lang/10-programs.md` says such a path resolves at run time and throws if it cannot
  be read. Two answers, and it is the user's: build the runtime resolve, or refuse a non-literal
  path where it is written.
- An autoload miss does not name the roots that were searched, which
  `docs/reference/lang/10-programs.md` § *`autoload`* says it does. What it prints is
  `E0303 … no matching declaration`, so a wrong root reads as a wrong class name.
- A second `namespace` declaration in one file parses and takes effect, which
  `docs/reference/lang/10-programs.md` § *Namespaces and `use`* does not allow. No rule fragment
  decides it either way; whether to refuse one is the user's call. (Carried from the previous
  session.)
