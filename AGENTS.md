# Working on MWL

MWL is a JIT-compiled, memory-safe language for web servers and the command line. It exists to run web
requests and CLI programs **securely and fast**.

This is the only file you always need to read. It carries the rules that are easy to get wrong, and names
the two places that route you to everything else.

**Every fact in this repository has exactly one home.** If two documents state the same thing, the one
named as the home is authoritative and the other is a bug — fix it rather than reconciling it in your head.

Any agent, any harness. `CLAUDE.md` at the root is a pointer to this file; nothing here is Claude-specific.

## Where to look

Do not read the docs tree breadth-first: most of it is reasoning you only need when you are about to
overturn a decision. Two things route you:

**1. Run `python tools/brief.py` first, every session.** One call: the plan's status block, a one-line map
of every milestone plus the lead of the current one, the one-line title and status of every ADR, what the
guard tests actually hold with their thresholds, and what exists on disk. It stores no facts — it slices
the live files and names each source, so it cannot go stale, and it says so loudly if a slice comes back
empty. It deliberately does not print each ADR's full rule or a milestone's full text — open the file it
names, or the plan at the line number it prints, for those.

**2. [docs/adr/README.md](docs/adr/README.md) § *Where to look*** is the topic → file routing table: one
row per topic, naming the one file that owns it. `python tools/brief.py --where <keyword>` prints just the
rows that match, which is usually faster than opening the file. That table lives there rather than here
because it grows a row per ADR, and this file is read in full at the start of every session.

**An ADR's body always states the current rule.** A later decision is folded into the earlier ADR's text,
never left as an overlay you have to apply while reading. If a body disagrees with a cross-link, the body
is the bug — fix it.

## The priority ordering

Highest first. A lower item is spent to buy a higher one, never the reverse. Reasoning and bounds:
[ADR 0004](docs/adr/0004-memory-for-simplicity.md).

1. **Security and request isolation** — not traded for anything.
2. **Correctness of language semantics** — PHP-compatible observable behaviour.
3. **Latency and throughput on the request path.**
4. **Simplicity** — of the language surface first, then of the implementation.
5. **Memory footprint** — last, and spent deliberately to buy any of the above.

When choosing between designs:

- Prefer the safer, faster or simpler one even when it holds more memory. MWL is **not** a low-footprint
  runtime; "allocates less" is not on its own a reason to change anything.
- If a change spends memory, **say what it spends** — per request or per task — in the doc comment or ADR
  that records it.
- Memory must stay attributable to a request and under an enforceable cap, and must be O(in-flight) rather
  than O(requests served). Growth with total traffic is a leak, not a trade-off.
- Bytes *moved* are not cheap. An allocation or extra cache miss on a hot path is a latency question
  (priority 3), not a footprint one.
- Saving memory at the cost of an invariant every future contributor must remember is the wrong direction —
  that is the account the unsafe modules are already drawing on.

## Ground rules enforced elsewhere

**One sentence per rule. If you need more than a sentence, it belongs in the ADR.** The linked file holds
the mechanism, the exact spellings rejected, and the reasoning.

### Implementation invariants

- **`unsafe` is forbidden workspace-wide**; only `mwl-runtime`, `mwl-codegen`, `mwl-stdlib` and
  `benches/abi-probe` opt down to `deny` with narrow, reasoned allows ([Cargo.toml](Cargo.toml)).
- **Nothing unwinds through a JIT frame** — every call returns a checked status
  ([0002](docs/adr/0002-error-propagation.md)).
- **Pure-Rust dependencies by default**, enforced by [deny.toml](deny.toml) in CI; a C dependency must pass
  [0051](docs/adr/0051-standard-library-tiers.md) § 4's two questions.
- **Architecture assumptions are tested, not remembered** — [benches/abi-probe/](benches/abi-probe/) guards
  them on every CI run; if one fails, revisit the ADR it points at rather than the threshold.
- **Every third-party notice MWL owes is generated, committed and embedded in the binary**
  ([0065](docs/adr/0065-third-party-attribution-and-mwl-info.md)).

### The type system

- **Nothing is untyped, and no type ever changes by itself** — every binding declares a type, `mixed` is
  the one unchecked position, and there is no `resource` type
  ([0007](docs/adr/0007-explicit-type-system.md)).
- **`var $name = expr;` infers a local's type from its initializer and fixes it forever** — a bare
  array-literal initializer is the one shape it refuses
  ([0037](docs/adr/0037-var-local-type-inference.md)).
- **`as` is the only conversion spelling**; PHP's `(int)$x` does not parse
  ([0034](docs/adr/0034-legacy-cast-syntax-rejected.md)).
- **`as ?T` converts without throwing, yielding `null` where `as T` would throw**
  ([0066](docs/adr/0066-nullable-conversion-operator.md)).
- **A condition is the one place a value is tested without `as`**, resolving PHP's full truthy table
  ([0035](docs/adr/0035-truthy-boolean-context.md)).
- **`string` is guaranteed-valid UTF-8; binary data is the separate `bytes` type** — only its default
  length granularity is still open ([0009](docs/adr/0009-string-and-bytes.md) § 2).
- **`decimal` is a scalar, not a class**, and `decimal ⊕ float` is a compile error
  ([0054](docs/adr/0054-decimal-scalar-type.md)).
- **Enums are a closed, named integer type**, never PHP's class-like construct
  ([0010](docs/adr/0010-enums-are-a-value-type.md)).
- **A `string`/`int` literal and a named enum case are each their own type**, unioned to declare a closed
  set ([0047](docs/adr/0047-literal-and-enum-case-types.md)).
- **`object` is the opaque top of every class type, and `{a: 1}` builds an anonymous methodless instance** —
  an inline `{name: T}` shape type is the one structurally checked exception
  ([0036](docs/adr/0036-anonymous-object-shapes.md)).
- **Every constructor must definitely assign every property it declares**
  ([0022](docs/adr/0022-definite-property-initialization.md)); `lateinit` is the one opt-out
  ([0038](docs/adr/0038-lateinit-property-modifier.md)).

### The language surface

- **Every function is a method, every constant a class constant** — no free function, no global constant
  ([0011](docs/adr/0011-functions-and-constants-are-class-members.md)).
- **`static` marks a class member; nothing else holds state behind a function's back**
  ([0008](docs/adr/0008-static-and-global.md)).
- **No variable is ever populated by the host** — PHP's superglobals become `Core` accessor classes
  ([0012](docs/adr/0012-no-superglobals.md)).
- **Nothing gets a second runtime-reachable name** — a compile-time `type` alias is the one exception
  ([0015](docs/adr/0015-no-name-aliasing.md)).
- **There is no `trait`** — shared behavior is an interface method with a body, shared state is
  `implements Interface by $field;` delegation
  ([0043](docs/adr/0043-interface-default-methods-and-delegation-replace-traits.md)).
- **Ordering two objects requires the global `Comparable` interface**, with no property-walk fallback
  ([0013](docs/adr/0013-comparable-interface.md)).
- **A property access runs its own hook, then a declared `PropertyObserver`** — an undeclared property is
  always a hard error, and `__call`/`__callStatic` do not exist
  ([0014](docs/adr/0014-property-observer.md)).
- **`Stringable` replaces `__toString`, and MWL has no destructors, `__debugInfo` or `__set_state`**
  ([0028](docs/adr/0028-closing-the-remaining-magic-methods.md)).
- **`callable` is satisfied by exactly one shape of value, a closure, and there is no `__invoke`**
  ([0027](docs/adr/0027-callable-is-closures-only.md)).
- **`fn` is the only closure literal, closures have no `use` clause, and `callable` is the only type name**
  ([0031](docs/adr/0031-callable-is-the-only-closure-type.md)).
- **`Iterable`/`Iterator` are the only iteration interfaces, and generators lower to a state machine** —
  no `ArrayAccess`, no `Countable` ([0053](docs/adr/0053-iteration-and-generators.md)).
- **`clone` is PHP's shallow copy; `serialize` shares one graph-copy operation with the `spawn` boundary** —
  neither has a customization hook
  ([0023](docs/adr/0023-clone-serialize-and-cross-boundary-copy.md)).
- **`#[...]` attributes are shape-literal metadata, never a declared attribute class**
  ([0046](docs/adr/0046-attributes-shape-literal-metadata.md)).
- **`require` is the only same-frame file-inclusion construct**
  ([0021](docs/adr/0021-single-file-inclusion-construct.md)).
- **A name reaches its file through a compile-time `autoload` declaration, never a runtime loader**
  ([0061](docs/adr/0061-compile-time-autoload-and-program-discovery.md)).
- **`&&`/`||` are the only logical connectives**; `and`/`or`/`xor` do not parse
  ([0045](docs/adr/0045-and-or-xor-keyword-operators-rejected.md)).
- **`<?mwl` is the only code-mode open tag and `exit` the only termination keyword** — `<?=` is sugar for
  `<?mwl echo`, not a second tag ([0049](docs/adr/0049-single-open-tag-and-single-exit-keyword.md)).
- **`[...]` is the only destructuring spelling**
  ([0050](docs/adr/0050-list-destructuring-spelling-rejected.md)).
- **Identifier casing is a hard compiler error with no suppression**, checked on the leading character only
  ([0029](docs/adr/0029-identifier-casing-is-checked.md)).
- **No identifier may start with `_`, and the constructor is spelled `constructor`**
  ([0030](docs/adr/0030-no-leading-underscores-constructor-spelling.md)).
- **Nothing depends on the case something was typed in, or on the filesystem's opinion of it**
  ([0062](docs/adr/0062-case-sensitivity-is-a-compiler-property.md)).

### Security and isolation

- **Untrusted input carries a `tainted` qualifier that `Core` sinks refuse until it is laundered** — the
  HTML sink additionally auto-escapes by default
  ([0024](docs/adr/0024-taint-tracking-for-injection-sinks.md)).
- **A confidential value carries a `secret` qualifier that output, logs, dumps, `Throwable` messages and
  serialization all refuse** ([0033](docs/adr/0033-secret-qualifier-for-confidential-values.md)).
- **An extension's manifest can only tighten the qualifier analysis, never loosen it**
  ([0055](docs/adr/0055-extension-qualifier-declarations.md)).
- **Extensions are sandboxed wasm, never `dlopen`** ([0003](docs/adr/0003-extension-system.md)).
- **An isolate shares nothing but compiled code, and spends its parent's budget**
  ([0006](docs/adr/0006-isolated-script-execution.md)).
- **Compiled code is the only thing a request shares with any other**
  ([0017](docs/adr/0017-hot-reload-without-restart.md)).
- **Cross-request state is explicit: `Core\Cache` is per-core, copied in and out, charged to the core**
  ([0059](docs/adr/0059-cross-request-state-is-explicit.md)).
- **`Core\Process` is the only way to run another program, and it never accepts a shell string**
  ([0044](docs/adr/0044-core-process-argv-only-no-shell.md)).
- **An outbound URL is a sink, and the connection is made to a pinned address**
  ([0058](docs/adr/0058-outbound-request-policy.md)).
- **Regex runs on a linear-time engine by default**, under a step budget that throws
  ([0056](docs/adr/0056-regex-engine-policy.md)).
- **A fatal error never reaches an ordinary `catch`** — it escalates through a zero-retry, reserved-budget
  handler ladder ([0020](docs/adr/0020-error-escalation-ladder.md)).
- **Four doors stay shut: no FFI, no stream wrappers, no cross-request state, no `eval`** — none has an
  opt-in ([0052](docs/adr/0052-closed-doors.md)).
- **A closed four-entry roster of application-layer security protocols lives in `Core`**
  ([0060](docs/adr/0060-application-security-protocols.md)).

### Runtime, tooling and the standard library

- **A stdlib candidate is placed by six ordered tests, and only Tier 0 may claim the `Core` prefix**
  ([0051](docs/adr/0051-standard-library-tiers.md)).
- **Every `Core` member has the same shape: subject first, one trailing options shape, nothing mutates,
  failure throws, absence is `?T`** ([0063](docs/adr/0063-core-api-conventions.md)).
- **The compiler validates a closed list of `Core` intrinsics' literal arguments during checking**
  ([0057](docs/adr/0057-intrinsic-literal-folding.md)).
- **Reflection and AST parsing are built into `Core`, not left to extensions**
  ([0019](docs/adr/0019-reflection-and-ast-parsing-are-core-features.md)).
- **One database API: a connection is named in root-owned config, every statement is prepared, and a
  transaction is a closure** ([0067](docs/adr/0067-core-db.md)).
- **Configuration is TOML, in a root-owned `mwl.toml`, read once at boot**
  ([0064](docs/adr/0064-configuration-file-format.md)); it states defaults, not ceilings
  ([0005](docs/adr/0005-config-changeability.md)).
- **Coverage, tracing and profiling are always-emitted, flag-gated probes, never a second compiled tier**
  ([0018](docs/adr/0018-coverage-tracing-and-profiling-as-safepoint-shaped-probes.md)); GC and spawn events
  are instrumented in their own routines, off the hot path
  ([0041](docs/adr/0041-timeline-export-and-gc-spawn-trace-events.md)).
- **The on-disk artifact cache is one immutable file per unit, verified before any page is made executable**
  ([0042](docs/adr/0042-on-disk-artifact-cache-format.md)).
- **A portable single-file executable ships source appended to the host binary**
  ([0048](docs/adr/0048-portable-single-file-executables.md)).
- **`mwl fmt` has one unconfigurable style, never reflows, and is never wired into `mwl check`**
  ([0039](docs/adr/0039-canonical-code-formatting.md)).
- **`mwl-syntax` exposes a second, lossless, error-recovering parse entry point for editor tooling only**
  ([0040](docs/adr/0040-vscode-deep-tooling-and-resilient-parsing.md)); IDE smarts live once, in `mwl-lsp`
  ([0016](docs/adr/0016-ide-integration.md)).
- **A wasm32 browser target is a second codegen backend behind the same IR, not a second language**
  ([0025](docs/adr/0025-wasm-browser-target.md)).
- **Performance history is callgrind instruction counts on a dedicated Linux runner, never raw wall-clock
  across machines** ([0026](docs/adr/0026-performance-measurement-methodology.md)).

## Commands

**A shell runs programs; it never carries file content.** Read, search, create and edit files with the
Read, Grep, Glob, Write and Edit tools — never `cat`, `head`, `tail`, `sed -n`, `grep`, `ls`, `find`, or a
heredoc that writes a file. This is not a style preference. A shell tool call is one `-c` string that the
shell *parses* before it runs anything, so an apostrophe in a doc sentence, a backtick in a commit message
or an unbalanced heredoc terminator fails the whole call with `unexpected EOF while looking for matching '`
— the command never executed, and nothing tells you which quote was at fault. The dedicated tools pass
content as JSON parameters with no shell in the path, so that failure cannot occur. This repo makes the
problem worse than most: prose full of apostrophes, backtick-quoted identifiers everywhere, and two shells
with incompatible quoting grammars (PowerShell primary, Git Bash for the Bash tool).

Use a shell for what it is for — `cargo`, `git`, `python tools/brief.py`, `wsl.exe`. When one of those
needs a multi-line argument, put the text in a file with the Write tool and pass the path: `git commit -F
<file>`, never an inline heredoc or a `-m` string spanning lines.

**An Edit the tool cannot express goes through `python tools/splice.py <target> <old> <new>`** — write
the exact old and new blocks to files under `.agent-tmp/` (gitignored; create it if it is not there) with
the Write tool and let that script swap them. It matches plain text and refuses anything but exactly one
hit, so a stale anchor is an error rather than a silent wrong edit. Never reach for a `sed`/`python - <<'PY'`
one-liner instead: a heredoc is a shell string, so it eats the backslashes and apostrophes this
repository's Rust and prose are full of.

**One shell call runs one command, and its exit status is the last one's.** Do not `;`-chain several probes
into a single call to save a round trip. A chain reports only the final command's status, so a probe that is
*allowed* to fail — `ls` on a directory that may not exist returns 2 — marks the whole call failed while
holding a complete result. `2>/dev/null` does not help: it suppresses the message, not the status. When a
command may legitimately fail, either give it its own call or end it with `|| true`, and put a `&&` between
steps that genuinely depend on each other.

```sh
cargo build                                                    # debug; deps still built at opt-level 2
cargo test                                                     # unit + integration
cargo clippy --all-targets -- -D warnings
cargo fmt --check
cargo test --release -p mwl-abi-probe                          # cost guards (skipped in debug)
cargo test --release -p mwl-abi-probe --features wasm-probe     # + sandbox probes (pulls in Wasmtime)
```

### Fuzzing and callgrind on Windows: use WSL

`cargo-fuzz` (the `fuzz/` crate) needs libFuzzer, and `valgrind`/`callgrind`
([ADR 0026](docs/adr/0026-performance-measurement-methodology.md)) has no native Windows build at all — do
both in WSL. From a Windows shell, `wsl.exe -- bash -lc "<command>"` runs a command in the default WSL
distro, which mounts the repo at `/mnt/<drive>/<repo>`. One-time setup in that distro:

```sh
sudo apt-get update && sudo apt-get install -y build-essential clang valgrind
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
source "$HOME/.cargo/env"
rustup toolchain install nightly
cargo install cargo-fuzz --locked
```

Then, from `/mnt/<drive>/<repo>` (not `fuzz/` itself — cargo-fuzz expects the parent directory):
`cargo +nightly fuzz run lex -- -max_total_time=300` (and `parse` likewise). CI's `fuzz-smoke` job runs both
for 60s on every push.

For the instruction-count leg: `cargo build --release -p mwl-abi-probe --example callgrind_spike`, then
`valgrind --tool=callgrind --callgrind-out-file=/tmp/cg.out ./target/release/examples/callgrind_spike`.

**A hand-written refcount protocol is where a leak hides, so check one before you commit it.**
`wsl.exe -- bash /mnt/<drive>/<repo>/tools/leak-check.sh <fixture> …` runs `valgrind --leak-check=full` over the
`.mwl` files you name. Run it for **any** new refcount edge, against a fixture that actually exercises it —
this repository's one real leak went unnoticed until a fixture happened to declare a refcounted local
inside a loop.

## Writing docs here

The docs are optimised for an agent that reads one file and starts working. Keep them that way:

- **State a fact once.** Put it in the home named in *Where to look*, and link to it from everywhere else.
  Summarising an ADR into another document creates a second copy that will silently go stale.
- **Never quote a measured number outside the ADR that owns it.** Numbers live with their guard test.
- **Fold, never overlay.** When a decision changes, edit the ADR that stated it so its body is true, and
  leave a one-line cross-link. Never add a paragraph to one file describing what another file changed —
  that is what makes a reader apply patches in their head, and it is how ADR 0007 came to carry eighteen of
  them at once. `git log` is the changelog.
- **Front-load.** Decision first, reasoning below it. Assume the reader stops after the first screen.
- A choice that would be expensive to reverse gets its own numbered ADR if the reasoning is subtle or
  contested; otherwise a paragraph in *Decisions taken at project start* in
  [docs/adr/README.md](docs/adr/README.md).
- Crates for later milestones are created when their milestone starts, not left sitting empty.

### Length targets, and why nothing enforces them

These are the shapes that keep a doc readable at a glance. **Every one is guidance addressed to you, not
a check.** Nothing in this repository, in `tools/brief.py`, or in CI measures a line, a field or a file
against a number, and nothing ever will:

| Thing | Aim for |
|---|---|
| One field of the plan's status block | ~400 bytes, one overwritten paragraph |
| One `### Mn — title` milestone heading | one short line |
| One ADR index **Decision** cell | one sentence, ~160 bytes |
| One AGENTS.md ground-rule bullet | one sentence, plus the link |
| One *Decisions taken at project start* title | one short phrase |
| One guard test's name + bounds | one line |
| `docs/agent/handoff.md` | ~80 lines — state, not a changelog |

Write to the target, and if a line lands a little over, **leave it**. This used to be a hard check that
failed CI, and the cost was not the bytes: it was five and ten iterations per session spent shaving prose
to clear a tripwire, at the end of a session, when the real work was already done.

The one structural rule that *does* still matter is not about length: `tools/brief.py` may only print text
bounded by a **count of entities** — one line per milestone, per ADR, per guard test — never by a length of
prose.

## Session workflow

Every session runs the same five steps, in this order, and **stops**:

1. **Orient.** `python tools/brief.py`, this file, then `docs/agent/handoff.md` for what to pick up.
2. **Do the work.** One focused slice. Keep it small enough to finish.
3. **Verify what you touched** — `cargo build`, `cargo test`, `cargo clippy --all-targets -- -D warnings`,
   `cargo fmt --check`, plus whatever the change specifically warrants (a `valgrind` run for a new refcount
   edge, per *Commands*). **This is the only place verification happens.**
4. **Write the docs and the handoff.** Update the plan's status block and any doc the change invalidates,
   then overwrite `docs/agent/handoff.md` with where the work stands now.
5. **Commit everything.** Then you are done.

**After step 5, stop.** Do not re-run `cargo build`/`test`/`clippy`/`fmt`, do not re-read the digest, do
not re-check a doc against a length. Writing prose cannot break a build, so there is nothing a second test
run could discover. If step 4 or 5 turned up a real problem, fix it and re-verify *that* — otherwise the
session is over.

## Keep work small, commit your work

Step 5 above, in detail:

- Always commit your work when a step is done. You don't need to review the history first — commit
  everything that has changed.
- The handoff is `docs/agent/handoff.md`: **overwrite it**, never append, so it describes where the work
  stands now rather than the path taken to get here. Its shape is in
  [docs/agent/session-prompt.md](docs/agent/session-prompt.md). Then show the user the same prompt in chat.
- The docs accumulate rationale bloat as ADRs are added. Periodically — the user fires this by hand, never
  you automatically — re-run the pass in [docs/agent/doc-cleanup.md](docs/agent/doc-cleanup.md).
- Every time we add, change or remove a feature, decide and say what the tradeoffs are in performance,
  memory, usability and simplicity for developers using the language. If there are large tradeoffs, notify
  the user and ask for agreement before proceeding. If there are only benefits, go ahead.
- [docs/implementation-plan.md](docs/implementation-plan.md)'s leading status block has a **fixed field
  set** — `Status`, `Done`, `On disk`, `Toolchain`, `ADR slices landed`, `Open now`, `Blocking`. Overwrite
  a field in place each session; never append a paragraph, and never add a field name. Session-by-session
  history lives in `git log`; per-file known-gap detail belongs in that crate's own module doc comment, not
  in the plan.
