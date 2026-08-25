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
of every milestone plus the lead of the current one, **one line per source module plus the definitions
sessions most often grep for**, what the guard tests actually hold with their thresholds, and what exists
on disk. It stores no facts — it slices the live files and names each source, so it cannot go stale, and it
says so loudly if a slice comes back empty. It deliberately does not print a milestone's full text or a
module's full doc comment — open the file it names, or the plan at the line number it prints, for those.
The ADR index is a count by default, because the ground rules below already carry one bullet per decision;
`--adrs` prints the whole table, and any ADR that is not Accepted always prints.

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

- **MWL is built first for platforms that run code, and data, they do not control** — so the framework and
  the dependency story outrank new breadth, the pitch is isolation and qualifiers rather than speed, and no
  document may claim PHP compatibility ([0080](docs/adr/0080-the-audience-mwl-is-built-for.md)).
- **`unsafe` is forbidden workspace-wide**; only `mwl-runtime`, `mwl-codegen`, `mwl-stdlib` and
  `benches/abi-probe` opt down to `deny` with narrow, reasoned allows ([Cargo.toml](Cargo.toml)).
- **Nothing unwinds through a JIT frame** — every call returns a checked status
  ([0002](docs/adr/0002-error-propagation.md)).
- **Pure-Rust dependencies by default**, enforced by [deny.toml](deny.toml) in CI; a C dependency must pass
  [0051](docs/adr/0051-standard-library-tiers.md) § 4's two questions.
- **Domain logic is an existing first-class Rust crate; compiler passes and scheduler primitives are ours** —
  anything with an external specification is a dependency, and if no crate exists the feature is not built
  ([docs/adr/README.md](docs/adr/README.md) § *Decisions taken at project start*).
- **SIMD comes from a dependency that dispatches at runtime, never from an intrinsic we wrote or a
  `target-cpu` flag, and the JIT emits scalar code by design**
  ([docs/adr/README.md](docs/adr/README.md) § *Decisions taken at project start*).
- **Architecture assumptions are tested, not remembered** — [benches/abi-probe/](benches/abi-probe/) guards
  them on every CI run; if one fails, revisit the ADR it points at rather than the threshold.
- **Every third-party notice MWL owes is generated, committed and embedded in the binary**
  ([0065](docs/adr/0065-third-party-attribution-and-mwl-info.md)).
- **Dependencies stay current, and from 0.1.0 a break in one is absorbed rather than forwarded to MWL
  programs** — until then, update anything freely; the sweep is a pass the *user* fires, never an agent
  ([0068](docs/adr/0068-dependency-currency-and-the-version-contract.md),
  [docs/agent/dependency-update.md](docs/agent/dependency-update.md)).

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
- **`string` is guaranteed-valid UTF-8 and counts grapheme clusters; binary data is the separate `bytes`
  type, counting bytes** ([0009](docs/adr/0009-string-and-bytes.md)).
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
- **`==` is the only equality operator, it never converts, and two statically disjoint types do not
  compile** — strings, arrays and objects each take the strict reading
  ([0090](docs/adr/0090-one-equality-operator-and-disjoint-types-do-not-compile.md)).
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
- **A bidirectional control that opens a scope and never closes it is a compile error in source and is
  substituted at both output sinks** ([0087](docs/adr/0087-unbalanced-bidi-is-rejected-at-every-boundary.md)).

### Security and isolation

- **Untrusted input carries a `tainted` qualifier that `Core` sinks refuse until it is laundered** — the
  HTML sink additionally auto-escapes by default
  ([0024](docs/adr/0024-taint-tracking-for-injection-sinks.md)).
- **A sink is a parameter whose content becomes an instruction, and an unclassified one refuses** — `echo`
  binds to the terminal sink everywhere but an HTTP request, where a body is one typed `Core\Response`
  member ([0088](docs/adr/0088-a-sink-is-an-instruction-and-the-default-refuses.md)).
- **A confidential value carries a `secret` qualifier that output, logs, dumps, `Throwable` messages and
  serialization all refuse** ([0033](docs/adr/0033-secret-qualifier-for-confidential-values.md)).
- **An extension's manifest can only tighten the qualifier analysis, never loosen it**
  ([0055](docs/adr/0055-extension-qualifier-declarations.md)).
- **Extensions are sandboxed wasm, never `dlopen`** ([0003](docs/adr/0003-extension-system.md)).
- **An isolate shares nothing but compiled code, and spends its parent's budget**
  ([0006](docs/adr/0006-isolated-script-execution.md)).
- **A WebSocket or SSE connection is its own root isolate, opened by naming a file**, and `Core\Topic`
  closes a slow subscriber rather than blocking a publisher
  ([0083](docs/adr/0083-persistent-connections-are-isolates.md)).
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
- **HTTP defaults are safe inbound and finite outbound** — secure headers and closed CORS with nothing
  configured, and no spelling for an unbounded outbound wait
  ([0074](docs/adr/0074-http-defaults-safe-and-finite.md)).
- **`Core\RateLimit` limits what only the application knows** — per account, per tenant — while edge and
  flood limiting stay the proxy's ([0075](docs/adr/0075-core-ratelimit.md)).

### Runtime, tooling and the standard library

- **A stdlib candidate is placed by six ordered tests, and only Tier 0 may claim the `Core` prefix**
  ([0051](docs/adr/0051-standard-library-tiers.md)).
- **Every `Core` member has the same shape: subject first, one trailing options shape, nothing mutates,
  failure throws, absence is `?T`** ([0063](docs/adr/0063-core-api-conventions.md)).
- **Arrays combine by the member's name, never by a key's type** — `overlay`/`underlay`/`appendAll`, no
  `merge`, and `array + array` does not compile
  ([0069](docs/adr/0069-array-combination-is-key-type-independent.md)).
- **The compiler validates a closed list of `Core` intrinsics' literal arguments during checking**
  ([0057](docs/adr/0057-intrinsic-literal-folding.md)).
- **Reflection and AST parsing are built into `Core`, not left to extensions**
  ([0019](docs/adr/0019-reflection-and-ast-parsing-are-core-features.md)).
- **One database API: a connection is named in root-owned config, every statement is prepared, and a
  transaction is a closure** — and a released connection rejoins a per-core pool only after a reset that is
  a security boundary ([0067](docs/adr/0067-core-db.md), § 13 for the pool).
- **A dependency is a content-addressed archive resolved by minimal version selection, no package code runs
  before your program does, and a package's capabilities are granted one line at a time rather than
  inherited** ([0081](docs/adr/0081-packages-are-digests-resolution-is-a-maximum.md)).
- **MWL ships its own framework, split by ADR 0051's six tests** — privileged halves in `Core`, the
  opinionated layer as the `mwl/web` package, no ORM and no runtime container
  ([0082](docs/adr/0082-the-first-party-framework.md)).
- **A background job is a row in a `Core\Db` table, so an enqueue commits with the write that caused it**,
  and delivery is at-least-once with bounded retries
  ([0084](docs/adr/0084-durable-background-jobs.md)).
- **The OpenAPI document is generated while compiling, and an `#[Api]` that contradicts the code is a
  compile error** ([0085](docs/adr/0085-openapi-is-generated-from-the-route-table.md)).
- **`#[Json\Derive]`/`#[Db\Derive]` generate a codec from a class's declared properties, and a failed decode
  reports every bad field at once** — a compiler-recognized attribute is matched by name, unlike
  `Core\Attributes` retrieval ([0071](docs/adr/0071-derived-codecs.md)).
- **Configuration is TOML, in a root-owned `mwl.toml`**
  ([0064](docs/adr/0064-configuration-file-format.md)); it states defaults, not ceilings
  ([0005](docs/adr/0005-config-changeability.md)).
- **A run mode is `development` or `production`, defaults to production, and only selects the defaults of
  four named directives** — no environment variable is ever read for it
  ([0091](docs/adr/0091-run-mode-is-two-values-a-ceiling-and-a-list-of-defaults.md)).
- **Every developer-facing output is one closed record the sink in force renders as plaintext, JSON or
  HTML** — no call site names a format, and a dump reaches a response body only in development mode
  ([0092](docs/adr/0092-one-diagnostic-record-three-renderings.md)).
- **`mwl ctl reload` replaces the whole config snapshot over a local socket — no control port, no token** —
  and a directive that still needs a restart is named in the result rather than ignored
  ([0078](docs/adr/0078-config-reload-and-control-socket.md)).
- **`mwl service` registers this binary with the platform's service manager, storing one verbatim argv, and
  the installer is a sink that refuses any subcommand but `serve`/`run`**
  ([0093](docs/adr/0093-a-service-is-one-stored-argv-and-the-installer-is-a-sink.md)).
- **`Core\Task::all`/`::map` return with nothing still running, and `afterResponse` keeps the request tree
  alive past the connection** — cancellation runs no user code
  ([0072](docs/adr/0072-core-task-structured-concurrency.md)).
- **Scheduled work is a `[[schedule]]` block firing a `spawn script`, never an API** — `scope` is mandatory
  ([0073](docs/adr/0073-scheduled-work-is-config.md)).
- **`#[Route]` builds the route table while compiling, and the router stops at matching**
  ([0077](docs/adr/0077-compile-time-routing.md)).
- **The terminal is a sink that substitutes control bytes visibly, styling is the `Cli\Text` value type,
  and `#[Command]` builds the argument table while compiling**
  ([0086](docs/adr/0086-core-cli-terminal-is-a-sink.md)).
- **The runtime exports what it already measures, and a metric label refuses `tainted`**
  ([0076](docs/adr/0076-observability-export.md)).
- **A test is a `#[Test]` method whose table is built while compiling, and every test is its own isolate** —
  assertions are generic, so a type-mismatched comparison never runs
  ([0079](docs/adr/0079-testing-is-a-language-feature.md)).
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
- **`mwl convert` is one deterministic rule table read through two modes, and an "identical" rewrite is one
  a differential case against the PHP oracle proves**
  ([0089](docs/adr/0089-convert-is-one-rule-table-with-two-modes.md)).
- **`mwl-syntax` exposes a second, lossless, error-recovering parse entry point for editor tooling only**
  ([0040](docs/adr/0040-vscode-deep-tooling-and-resilient-parsing.md)); IDE smarts live once, in `mwl-lsp`
  ([0016](docs/adr/0016-ide-integration.md)).
- **A wasm32 browser target is a second codegen backend behind the same IR, not a second language**
  ([0025](docs/adr/0025-wasm-browser-target.md)).
- **Performance history is callgrind instruction counts on a dedicated Linux runner, never raw wall-clock
  across machines** ([0026](docs/adr/0026-performance-measurement-methodology.md)).

## Commands

**A shell never carries file content into the tree.** Create and edit files with the Write and Edit tools
— never a heredoc, a `>` redirect or a `sed -i` that writes a file. This is not a style preference. A
shell tool call is one `-c` string that the shell *parses* before it runs anything, so an apostrophe in a
doc sentence, a backtick in a commit message or an unbalanced heredoc terminator fails the whole call with
`unexpected EOF while looking for matching '` — the command never executed, and nothing tells you which
quote was at fault. The dedicated tools pass content as JSON parameters with no shell in the path, so that
failure cannot occur. This repo makes the problem worse than most: prose full of apostrophes,
backtick-quoted identifiers everywhere, and two shells with incompatible quoting grammars (PowerShell
primary, Git Bash for the Bash tool).

**Reading and searching are a preference, not a prohibition.** Prefer Read, Grep and Glob: they need no
quoting, they return line numbers you can cite, and Grep takes a real regex without a shell mangling it.
A `grep`/`sed -n` through a shell is allowed where it is genuinely shaped better — a pipeline over a
*command's* output, a `wc -l` across a glob — because nothing is being written and a bad quote costs one
retry rather than a silent wrong edit.

Use a shell for what it is for — `cargo`, `git`, `python tools/brief.py`, `wsl.exe`. When one of those
needs a multi-line argument, put the text in a file with the Write tool and pass the path: `git commit -F
<file>`, never an inline heredoc or a `-m` string spanning lines.

**An Edit the tool cannot express goes through `python tools/splice.py <target> --patch <file>`** — write
one patch file under `.agent-tmp/` (gitignored; create it if it is not there) with the Write tool, holding
the old and new blocks between `<<<<<<< OLD` / `=======` / `>>>>>>> NEW` markers, and let that script swap
them. It refuses anything but exactly one match per block, so a stale anchor is an error rather than a
silent wrong edit, and a multi-block patch that half-matches leaves the file untouched. Never reach for a
`sed`/`python - <<'PY'`/`cat > f <<'EOF'` one-liner instead: a heredoc is a shell string, so it eats the
backslashes and apostrophes this repository's Rust and prose are full of. The exact format is in
[docs/agent/conventions.md](docs/agent/conventions.md).

**The plan's status block is edited with `python tools/plan.py --set "<field>" --from <file>`**, not by
hand — locating a field's exact bytes and splicing them was the single most expensive repeated action a
session performed.

**One shell call runs one command, and its exit status is the last one's.** Do not `;`-chain several probes
into a single call. A chain reports only the final command's status, so a probe that is *allowed* to fail —
`ls` on a directory that may not exist returns 2 — marks the whole call failed while holding a complete
result. `2>/dev/null` does not help: it suppresses the message, not the status. When a command may
legitimately fail, either give it its own call or end it with `|| true`, and put a `&&` between steps that
genuinely depend on each other.

**That is a limit on one call, not on one turn — independent calls go out together.** Issue every probe
whose input does not depend on another's result as several tool calls *in the same message*: four greps
locating a symbol, a Read of two files you already know you need, `git status` beside `cargo --version`.
They run concurrently and each keeps its own exit status, so nothing about the rule above is weakened.
This matters more than it looks: a session's wall clock is very nearly its number of turns times a
constant, and read-only probing is where the turns go — an unbatched orientation pass has cost this
repository a quarter of a session's clock, one `grep` at a time. Serialize only what genuinely depends on
a previous answer.

```sh
python tools/verify.py                                         # build + test + clippy + fmt, one call
python tools/verify.py -p mwl-ir                               # the same, scoped to one package
cargo test --release -p mwl-abi-probe                          # cost guards (skipped in debug)
cargo test --release -p mwl-abi-probe --features wasm-probe     # + sandbox probes (pulls in Wasmtime)
```

`verify.py` runs `cargo build`, `test`, `clippy --all-targets -- -D warnings` and `fmt --check` in that
order, stops at the first failure, and prints about ten lines when green — the four separately are four
calls and tens of thousands of tokens of output nobody reads once it passes. Every step's full output is
written to `.agent-tmp/verify-<step>.log` either way. It judges nothing: a step's own exit status is the
whole verdict.

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
| One module's `//!` first sentence | one line — it is the map's entry for that file |
| One Rust module | ~1,500 lines of code; past that, split it at a **spec-shaped** seam |
| One function | ~250 lines; past that, an arm with a rule of its own becomes a call |
| `docs/agent/handoff.md` | ~60 lines — state, not a changelog and not the playbook |
| `docs/agent/playbook.md` | no target; it grows a bullet at a time and that is correct |

**Spec-shaped** is the whole of the split rule: cut where an ADR or a grammar layer already draws a
line — `parser/{ty,expr,stmt,decl}`, `expr/{quals,operators,members}` — never at a line count, because a
seam the code's own decisions do not follow gets crossed by the next slice and the split has to be redone.
A directory's `mod.rs` then owes a **charter** in its `//!` header saying what it keeps and what it
delegates; without one it grows back into the file that was split.

Write to the target, and if a line lands a little over, **leave it**. This used to be a hard check that
failed CI, and the cost was not the bytes: it was five and ten iterations per session spent shaving prose
to clear a tripwire, at the end of a session, when the real work was already done.

The one structural rule that *does* still matter is not about length: `tools/brief.py` may only print text
bounded by a **count of entities** — one line per milestone, per ADR, per guard test — never by a length of
prose.

## Session workflow

Every session runs the same five steps, in this order, and **stops**:

1. **Orient in one call.**
   `python tools/brief.py && cat docs/agent/handoff.md docs/agent/playbook.md docs/agent/conventions.md`
   — the map, what to pick up, the trap list and the shape of everything this repository writes, in one
   turn rather than four. [playbook.md](docs/agent/playbook.md) is the traps; add a bullet to it when
   something new bites you. [conventions.md](docs/agent/conventions.md) is the shape — a commit message,
   a `.mwlt` case, a `Core` member's four edits, an ADR, a diagnostic. Read it instead of opening an
   existing example to copy; that lookup has the same answer every session.
2. **Do the work — a *group* of related slices, not one.** The handoff names the group and the file set
   it shares. Keep taking slices from it while the next one touches files already loaded, and stop at
   **four commits** or at the first slice that would need a fresh orientation — whichever comes first. A
   slice sharing no files with the group is the next session's, not this one's.
3. **Verify what you touched, once, at the end of the group** — `python tools/verify.py`, plus whatever
   the change specifically warrants (a `valgrind` run for a new refcount edge, per *Commands*). **This is
   the only place verification happens**, and a group shares one run: the build is the same build.
4. **Write the docs and the handoff, once for the whole group.** Update the plan's status block and any
   doc the change invalidates, then overwrite `docs/agent/handoff.md` with where the work stands now. It
   is *state* — a fact that will still be true in ten sessions belongs in the playbook, an ADR, or a
   crate's module doc instead. Naming the **next** group, and the file set it shares, is this step's job:
   you are the only one holding the context to decide it cheaply.
5. **Commit — one per slice, all after step 3 is green**, staging each slice's own files so `git log`
   still reads a slice at a time. Then you are done.

**After step 5, stop.** Do not re-run `cargo build`/`test`/`clippy`/`fmt`, do not re-read the digest, do
not re-check a doc against a length. Writing prose cannot break a build, so there is nothing a second test
run could discover. If step 4 or 5 turned up a real problem, fix it and re-verify *that* — otherwise the
session is over.

## Keep each slice small, commit every one of them

Step 5 above, in detail:

- Always commit your work before you exit, and never leave a slice uncommitted. You don't need to review
  the history first — stage each slice's own files, and let the last commit sweep whatever is left.
- The handoff is `docs/agent/handoff.md`: **overwrite it**, never append, so it describes where the work
  stands now rather than the path taken to get here. Its shape is in
  [docs/agent/session-prompt.md](docs/agent/session-prompt.md). Then show the user the same prompt in chat.
- **The playbook is the opposite file.** [docs/agent/playbook.md](docs/agent/playbook.md) is append-mostly:
  add a bullet when a trap costs you time, edit one when it stops being true, and otherwise leave it
  alone. Never reword it to say the same thing differently — this lore lived inside the handoff until it
  was two thirds of it, regenerated in full every session, and the rewording was the whole cost.
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
