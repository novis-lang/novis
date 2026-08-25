# ADR 0081 — A dependency is a digest, resolution is a maximum, and a package's authority is granted one line at a time

- **Status:** Accepted
- **Date:** 2026-08-24
- **Scope:** how third-party MWL code is named, fetched, pinned, resolved, verified, granted authority,
  published, retracted and vendored; the two source kinds (registry and git) and why only one of them is
  transitive; the manifest and lock files and how they avoid contradicting
  [0061](0061-compile-time-autoload-and-program-discovery.md) and
  [0064](0064-configuration-file-format.md). Not in scope: the registry's server implementation, the
  first-party framework's contents ([0082](0082-the-first-party-framework.md)), and the wasm sandbox an
  extension package runs in, which is [0003](0003-extension-system.md)'s.
- **Amends:** [0061](0061-compile-time-autoload-and-program-discovery.md) — its rejection of a manifest file
  stands untouched and § 7 below explains why this is not one: a fetched package is reached through an
  ordinary generated `autoload` declaration, so name resolution gains no new mechanism and no new
  invalidation edge. [0064](0064-configuration-file-format.md) § 4 — `mwl.toml` is still not a project
  manifest; `package.toml` is a *tool* input read by the `mwl` CLI, never by the runtime and never by name
  resolution, and § 8 states that boundary. [0055](0055-extension-qualifier-declarations.md) — its
  only-ever-tighten rule for extension manifests is generalised in § 4 to every package, source or wasm.
  [0051](0051-standard-library-tiers.md) § 1 — Tier 1's "first-party or third-party `.mwlx`" now has a
  defined distribution channel, the same one source packages use. [0068](0068-dependency-currency-and-the-version-contract.md)
  — its version contract governed MWL's own Rust dependencies; § 3 below adopts the same absorb-don't-forward
  discipline for MWL packages, which is what makes § 3's major-version rule affordable.
  [docs/plan/design.md](../plan/design.md) — the *Tooling* row's "package manager" gains a
  milestone.
- **Amended by:** none.
- **Relates to:** 0003, 0004, 0005, 0019, 0024, 0033, 0042, 0052, 0062, 0079, 0080, 0082

> **In short:** a package's **identity is its BLAKE3 digest**; a name is only a way to find one. Two kinds
> of source produce that digest — the **registry** (a name, `acme/http`) and a **git URL** — and the second
> is **root-only**: your own application may depend on a git URL, but a *published* package may depend only
> on registry packages, so no dependency of yours can ever drag in code from a URL you never saw. Versions
> resolve by **minimal version selection**: every dependency names a *minimum*, the build takes the highest
> minimum anyone asked for, and there is no solver, no backtracking and therefore **no unsolvable graph** —
> adding a dependency can never move an unrelated one. **Nothing in a package ever runs before your program
> does**: no install script, no build step, no macro, no code generation, so the entire post-install attack
> class has nowhere to execute. And **a dependency's authority is not ambient**: each package declares the
> capabilities it *requests*, the application grants them **per package, one line at a time**, the operator's
> `mwl.toml` caps them all ([0005](0005-config-changeability.md)), and a `Core` call needing a capability its
> own package was not granted is a **compile error** — so a formatting library that reaches for the network
> cannot, even on a machine where the network is granted. Integrity is a lockfile of digests plus an
> append-only **transparency log**, so a registry cannot serve one client different bytes than another
> without it being detectable.

## Context

- **This was the largest unhedged risk in the project.** Seventy-nine ADRs decided the language; the
  distribution of third-party code was one table cell in the plan reading `package manager`. For a language
  whose only real adoption risk is ecosystem ([0080](0080-the-audience-mwl-is-built-for.md)), that was the
  wrong thing to leave undesigned — and designing it late means designing it around whatever the ecosystem
  improvised in the meantime.
- **Two failure modes are named as things to avoid at any cost, and they have different causes.**
  *Dependency hell* — "your requirements could not be resolved" — is what a SAT solver over version *ranges*
  produces when a large graph carries conflicting constraints; it is a property of the resolution
  algorithm, not of the packages. *Supply-chain compromise* is a property of what a package is permitted to
  do: run code at install time, and hold whatever authority the process holds. Each needs its own
  structural answer, and § 3 and §§ 4–5 are those answers.
- **The incumbents demonstrate both failures precisely.** Composer resolves ranges with a solver and is
  where the phrase dependency hell lives in this ecosystem. npm's post-install scripts have executed
  attacker code in several of the most widely felt supply-chain incidents on record, and in every mainstream
  package system a dependency inherits the whole process's authority: a date-formatting library can open a
  socket because *the process* can. Nothing about that is inevitable — it is inherited from a time when
  packages were assumed friendly.
- **MWL is unusually well placed to fix the second one**, and almost by accident. It already has
  deny-by-default capabilities ([0005](0005-config-changeability.md)), a manifest discipline that may only
  tighten ([0055](0055-extension-qualifier-declarations.md)), no `eval`, no FFI, no stream wrappers
  ([0052](0052-closed-doors.md)), no macro system, and no code generation a package could hook. The pieces
  of a per-package authority model were all present; § 4 assembles them.
- **Go's module system is the strongest available evidence on resolution.** Minimal version selection
  removed dependency hell as a category rather than mitigating it, and its checksum database has held up as
  a transparency mechanism. Its costs are known too and § 3 takes them rather than pretending they are not
  there.
- **The registry and git question is not either/or.** The security properties come from the *digest*, the
  *lockfile* and the *capability grant* — none of which cares where bytes came from. What the source kind
  must decide is a narrower thing: how far into the graph an arbitrary URL may reach. § 2 is that rule.

## Decision

### 1. A package is an immutable, content-addressed source archive

- **Identity is a BLAKE3 digest** of the archive — the same hash [0042](0042-on-disk-artifact-cache-format.md)
  already uses, so the toolchain gains no second hash. Two archives with the same digest are the same
  package; two with different digests are different packages regardless of what they call themselves.
- **The archive holds MWL source and data files only.** No compiled artifacts, no shared objects, no
  binaries — with exactly one exception: an **extension package**, whose payload is a `.mwlx` wasm component
  and which is sandboxed by [ADR 0003](0003-extension-system.md) with qualifier declarations by
  [0055](0055-extension-qualifier-declarations.md). Both kinds are named, resolved, pinned, granted, logged
  and vendored identically; only the payload differs. There is no third kind, and a Tier 2 native subsystem
  is never a package ([0051](0051-standard-library-tiers.md) § 1).
- **A package may not contain an `mwl.toml`.** Server configuration is root-owned and deployment-scoped
  ([0064](0064-configuration-file-format.md)); a package that shipped one would be asking to configure the
  host it lands on. Fetching an archive containing one is an error naming the file.
- **A package's files are reachable only from within the package.** A `require` or an `autoload` path in a
  package that escapes the package's own directory — through `..`, an absolute path or a symlink — is a
  compile error. A package cannot read the application's tree, and the check is the same canonicalise-and-
  prefix-check `spawn script` already performs against its granted roots ([0006](0006-isolated-script-execution.md)).
- **A name is `vendor/name`**, both segments lowercase ASCII with `-` permitted, compared exactly —
  [0062](0062-case-sensitivity-is-a-compiler-property.md)'s rule, so a name that resolves on one operating
  system resolves on all three. `mwl` is reserved for first-party packages, as `Core` is reserved in the
  language ([0051](0051-standard-library-tiers.md) § 5).

### 2. Two sources for bytes, and only one of them is transitive

```toml
# package.toml
[dependencies]
acme/http      = "1.4.0"                                     # registry
acme/csv       = "0.3.0"

[dependencies.our-shared-lib]                                # git — root application only
git = "https://git.internal/team/shared-lib.mwl"
rev = "9f2c1e0b74a3d5f8c6e2b190a4d7f3c8e5b21094"
```

- A **registry dependency** is a name plus a minimum version. The registry maps it to a digest.
- A **git dependency** is a URL plus an exact commit `rev` — never a branch, never a tag, because neither is
  immutable. The fetched tree is archived and digested exactly as a registry artifact is, and the digest
  goes in the lockfile, so from the second fetch onward the two kinds are indistinguishable.
- **Only the root application may declare a git dependency.** A package published to the registry whose
  manifest names a git source is rejected at publish time, and a git dependency encountered below the root
  is a hard error naming the package that declared it. This is the rule that makes the hybrid safe rather
  than a loophole: the convenience of git — a fork, a patch you are testing, private code, a monorepo — is
  entirely a property of *your own* project, and your transitive graph stays inside a namespace with a
  transparency log and a retraction mechanism.
- **A private registry is a registry.** § 6's protocol is static signed files over HTTPS, so an
  organisation that cannot use the public one runs its own or mirrors it, and nothing above changes.

### 3. Resolution is a maximum, not a search

Every dependency names the **minimum version** it needs. The version selected for a package is the
**highest minimum any package in the graph asked for**. That is the whole algorithm.

- **There is no solver, no backtracking, and no unsolvable graph.** Resolution is a walk over the graph
  taking a maximum, so it always succeeds, it is fast, and it is deterministic without needing the lockfile
  to make it so — the lockfile records digests for *integrity*, not to pin a choice that would otherwise
  wobble.
- **Adding a dependency cannot move an unrelated one.** The version you get is one somebody explicitly
  asked for, never the newest thing published this morning, so a build that worked yesterday works today
  and a fresh clone matches CI.
- **Upgrades are explicit.** `mwl update <package>` raises a minimum in `package.toml` and shows the effect
  on the whole graph. `mwl outdated` reports what is available. Nothing upgrades itself.
- **A breaking release is a new package name.** Because a minimum is a floor and never a ceiling, MVS is
  only sound if a higher version is always acceptable. So the rule that keeps it sound is that a package
  that breaks its API publishes under a new name — conventionally the old name with a numeric suffix
  (`acme/http` → `acme/http2`) — carrying `supersedes = "acme/http"` for `mwl outdated` and `mwl audit`.
  The two coexist in one graph without conflict, because they are two packages. This is the real cost of
  no solver, it is paid by the *publisher* rather than by every consumer, and
  [ADR 0068](0068-dependency-currency-and-the-version-contract.md)'s absorb-don't-forward discipline —
  already MWL's own rule for its Rust dependencies — is what makes it rare enough to live with.
- **A known-bad version is retracted, not deleted.** A publisher marks a version retracted with a reason and
  a fixed-in version; resolution refuses to *select* it and names the fix, while the bytes remain fetchable
  forever so an existing lockfile still builds. `mwl audit` reads the registry's signed advisory feed,
  reports the minimum bump that clears each advisory, and exits non-zero for CI. This is the answer to
  MVS's one real weakness — that a patch nobody asked for does not arrive on its own.

### 4. A dependency's authority is granted per package, one line at a time

This is the section that matters most, and it inverts the assumption every mainstream package system makes.

- **A package declares what it requests.** Its manifest lists capability *names* from
  [ADR 0005](0005-config-changeability.md)'s roster — `net.connect`, `fs.read`, `db.connect`,
  `script.spawn`, and so on. The declaration is documentation and an upper bound on itself; it grants
  nothing.
- **The application grants, per package, explicitly.**

  ```toml
  [grants]
  "acme/http"       = ["net.connect"]
  "acme/csv"        = []                 # written by `mwl add`, and it stays empty
  ```

  **A package with no grant line gets nothing** — deny-by-default, the same posture
  [ADR 0005](0005-config-changeability.md) takes for the process as a whole. `mwl add` prints every
  capability requested by the package *and its whole transitive subgraph*, and writes the grant lines, so
  the authority a new dependency brings is visible in one diff at the moment it is introduced rather than
  discoverable by audit later.
- **The operator's `mwl.toml` still caps everything.** The effective set at any call site is the
  intersection of the operator's grant, the application's per-package grant, the package's own declaration,
  and any narrowing the enclosing isolate applied ([0006](0006-isolated-script-execution.md)). Every one of
  those may only ever *tighten* — [ADR 0055](0055-extension-qualifier-declarations.md)'s rule for extension
  manifests, now the rule for every package.
- **Enforcement is at compile time and costs nothing at run time.** Each source file belongs to exactly one
  package (§ 7's layout makes the mapping total), so a `Core` call requiring a capability that call site's
  package does not hold is a compile error naming the package, the capability and the grant line that would
  fix it. There is no runtime check to pay for and no dynamic path to escape through — a closure written in
  the application and *called* from a package still executes the application's code under the application's
  authority, which is both correct and what a reader expects.
- **What this buys, stated plainly:** a fully malicious package that reaches the compiler cannot open a
  socket, read a file, spawn a process, reach a database or spawn a script unless a human wrote its name in
  a grant line. The compromise of a transitive dependency — the incident class the incumbents keep having —
  degrades from *arbitrary action with the process's authority* to *arbitrary computation with no
  authority at all*.

### 5. Nothing in a package runs before your program does

- **No install scripts, no post-install hooks, no build step, no code generation, no macros.** There is no
  point in the fetch, resolve, verify or compile pipeline at which a package's code executes. The first
  time a line of a dependency runs is when your program calls it.
- **MWL had almost all of this already and it is worth being explicit about why**: there is no `eval`
  ([0052](0052-closed-doors.md)), attributes are inert shape literals ([0046](0046-attributes-shape-literal-metadata.md)),
  the only compiler-recognized attributes are a closed `Core`-owned list ([0071](0071-derived-codecs.md) § 1),
  and there is no macro expander for a package to hook. This section adds one rule to that — no build
  lifecycle, ever — and thereby closes the category.
- **A package that needs to generate code generates it into its own repository before publishing**, where a
  human reviews the output and the digest covers it. The generated file is source like any other.

### 6. Integrity: a lockfile, and a log the registry cannot lie to

- **`package.lock` records every package in the graph** — direct and transitive — with its name, selected
  version, source and digest. `mwl build --locked` (and every CI invocation) fetches nothing that is not in
  the lock and fails rather than updating it.
- **The registry publishes an append-only transparency log** of `name → version → digest`, a Merkle tree
  with signed checkpoints. Before a client uses an artifact it verifies the artifact's inclusion in the log
  and the log's consistency with the newest checkpoint the client has seen. A registry that serves one
  client different bytes than another must either fork the log — detected by the next client that checks
  consistency — or produce a signed statement contradicting one it already made. Compromising the registry
  therefore stops being silent, which is the property that matters.
- **A digest mismatch is a hard error, never a warning and never a re-fetch.**
- **`mwl vendor`** writes the whole resolved graph into the application tree, so a build needs no network at
  all. Vendored bytes are digest-checked on every build, so vendoring is a convenience rather than a
  second trust root.

### 7. A fetched package is reached by an ordinary `autoload` declaration

This is how the package system stays out of the language, and it is deliberately the least clever part of
the design.

- `mwl fetch` materialises the graph under `vendor/` and writes **one generated MWL source file**,
  `vendor/packages.mwl`, containing nothing but ordinary declarations:

  ```php
  <?mwl
  // Generated by `mwl fetch` from package.lock. Do not edit.
  autoload 'Acme\Http' from './acme-http-1.4.0/src';
  autoload 'Acme\Csv'  from './acme-csv-0.3.0/src';
  ```

- The application `require`s that file once. **That is the entire integration.** Name resolution is exactly
  [ADR 0061](0061-compile-time-autoload-and-program-discovery.md)'s — literal paths relative to the
  declaring file, no runtime loader, no walk-up search, no new invalidation edge, and the compiler learns
  nothing about packages in order to resolve a name.
- **The generated file is committed.** It is short, it is readable, and every change to the dependency graph
  shows up in a diff as a line of source rather than as a change in a tool's behaviour.
- **A package declares its own namespace prefix** in its manifest, and two packages claiming the same prefix
  is an error at fetch time naming both — the collision is caught when the graph is assembled rather than
  as a confusing redeclaration during compilation.

### 8. `package.toml` is a tool input, and that is why it is not the manifest 0061 rejected

[ADR 0061](0061-compile-time-autoload-and-program-discovery.md) rejected "a manifest file found by walking
up from the entry file", and [0064](0064-configuration-file-format.md) § 4 confirmed `mwl.toml` holds no
source-tree state. Both stand, because the distinction they draw is about *who reads the file*:

- **Neither the runtime nor name resolution ever reads `package.toml`.** Program semantics do not depend on
  it, no compiled unit's cache key includes it, and a program whose `vendor/` is already populated compiles
  identically whether the file is present, absent or malformed.
- It is read by the **`mwl` CLI** — `add`, `fetch`, `update`, `vendor`, `audit`, `publish` — which may find
  it by walking up from the working directory the way `git` finds `.git`, because a tool locating its own
  project is a different question from a language locating a declaration.
- It is TOML, `deny_unknown_fields`, for [0064](0064-configuration-file-format.md)'s reasons; the
  dependency is already in the tree.

### 9. Publishing

`mwl publish` uploads a source archive built from a clean checkout. The registry requires a verified
account with a second factor, refuses to overwrite a published `name@version` under any circumstance,
refuses an archive containing an `mwl.toml` or a git dependency, records the artifact in the transparency
log before it is downloadable, and records the manifest's requested capabilities so `mwl add` can show them
before a human grants anything. Names are first-come with a squatting policy the registry operator owns;
that policy is an operational document, not an ADR.

## Consequences

- **You operate a registry**: an index, an artifact store, a transparency log, an advisory feed, accounts,
  moderation and uptime. The protocol is deliberately static signed files so the serving side is cheap and
  mirrorable, but the *responsibility* is real and permanent, and it is the main cost this ADR incurs.
- **A publisher pays for breaking changes with a rename.** This is the sharp edge of § 3 and it will be
  unpopular with library authors used to `2.0.0`. It is the price of never showing a user an unsolvable
  graph, and the trade is deliberate: the cost falls on the few who break APIs rather than on everyone who
  consumes them.
- **A security patch does not arrive on its own.** MVS gives you the version somebody asked for, which is
  its whole point and also means a fix released this morning is not in your build until you ask. `mwl audit`
  plus retraction is the mitigation, and it must be genuinely good — a CI-friendly exit code and an exact
  minimum bump — rather than an afterthought.
- **Grant lines are friction, on purpose.** Adding a package means reading what it wants and writing a line.
  Some users will grant everything reflexively, exactly as some users run containers as root; the design
  cannot prevent that, only make it visible and diffable. What it does prevent is authority arriving
  *without* anybody writing anything.
- **A package can never be a launderer or a sink.** [ADR 0024](0024-taint-tracking-for-injection-sinks.md)
  § 3 already restricts qualifier removal to `Core` members whose contract names a sink, and
  [0051](0051-standard-library-tiers.md) test 2 keeps launderers in Core. So a package can compute over a
  `tainted` value and hand it back still `tainted`, and no third-party code can ever be the thing that
  declares data safe. This is a significant constraint on what a package can be, and it is correct.
- **Per-package capability checking needs the file → package map to be total**, which § 7's layout provides
  for fetched code. A file that belongs to no package — the application's own sources — is the application,
  and the application's grants are its own `[grants]` block plus the operator's ceiling.
- **First-party packages are subject to all of it.** `mwl/web` ([0082](0082-the-first-party-framework.md))
  resolves, locks, logs and is granted exactly like anyone else's package. That is deliberate: a registry
  whose maintainers do not depend on it does not stay good.

## Alternatives rejected

- **A SAT solver over version ranges (Composer, npm, Cargo).** More expressive — a range can say "not 2.3.1,
  it is broken" — and universally familiar. Rejected because it reintroduces the exact experience this ADR
  exists to prevent: unsolvable graphs, slow resolution, and an error message that blames the user for a
  conflict between two libraries they have never read. Retraction (§ 3) covers the one case ranges express
  that minimums cannot.
- **Exact pins only, no minimums.** Maximally predictable and trivially auditable. Rejected because every
  transitive upgrade becomes a manual sweep across the whole graph, which makes security patches propagate
  slowly — the failure mode § 3 is already accused of, made worse.
- **Git-only, no registry (the Go pre-modules and early Deno position).** Nothing to operate, no namespace
  to police. Rejected because the transparency log, retraction, the advisory feed and `mwl audit` all need
  a namespace with an authority behind it, and because "every URL in your transitive graph is a trust root"
  is precisely the property § 2 exists to remove.
- **Registry-only, with git repositories mirrored into it as named packages.** One resolution path for
  everyone and a slightly smaller design. Rejected as too rigid for the private and monorepo cases that
  every serious adopter has, and unnecessary: § 2's root-only rule already confines git's blast radius to
  the project that opted into it.
- **Ambient capabilities — grant the process, let every package inherit.** What every mainstream system
  does, and far less friction. Rejected outright: it is the mechanism by which a compromised transitive
  dependency becomes a breach, and refusing it is one of the few things MWL can offer that no incumbent
  can retrofit ([0080](0080-the-audience-mwl-is-built-for.md) § 2).
- **Allowing a build lifecycle for "packages that genuinely need it".** Every system that has permitted this
  has been exploited through it. There is no such package in a language with no FFI and no native
  compilation step available to userland ([0052](0052-closed-doors.md)), so the exception would exist only
  to be abused.
- **Publishing compiled artifacts.** Faster cold builds, and the on-disk cache format already exists
  ([0042](0042-on-disk-artifact-cache-format.md)). Rejected: a compiled artifact is unreviewable, and
  source distribution is what makes "the digest is the identity" a statement about code a human could read
  rather than about bytes nobody can.

## Verification

- **Resolution:** a graph where two packages require different minimums of a third selects the higher, and
  no other package's version changes. A graph with 500 packages resolves without backtracking. There exists
  no manifest for which resolution reports a conflict — asserted by a property test over generated graphs
  ([0079](0079-testing-is-a-language-feature.md)'s property testing, once it exists).
- **Git confinement:** a registry package whose manifest names a git dependency is refused at publish; the
  same package placed in `vendor/` by hand is refused at fetch, naming the package that declared it.
- **Capabilities:** a package that calls `Core\Http::get` with no `net.connect` grant fails to compile,
  naming the package, the capability and the grant line; granting it in the application's `[grants]` makes
  the same program compile; the operator's `mwl.toml` withdrawing `net.connect` makes it fail again. A
  closure defined in the application and invoked from a package runs under the application's grants,
  asserted by a fixture that would not compile if the mapping were by *caller* rather than by *code*.
- **No execution before the program:** a package containing a top-level statement with an observable effect
  produces no effect from `mwl fetch`, `mwl build` or `mwl vendor` — only from a call.
- **Escape attempts:** a package whose `require` or `autoload` path escapes its directory through `..`, an
  absolute path or a symlink is a compile error, mirroring
  [0006](0006-isolated-script-execution.md)'s existing root-check suite.
- **Integrity:** a tampered artifact fails the digest check and is not used, and the failure names the
  package; an artifact absent from the transparency log is refused; a log checkpoint inconsistent with a
  previously seen one is refused and reported as a registry fork rather than as a network error.
- **Reproducibility:** `mwl build --locked` on a clean machine with an empty cache produces byte-identical
  compiled units to the machine that wrote the lock, on all three platforms.
- **Integration:** a fetched package's classes resolve through `vendor/packages.mwl` with no compiler change
  — asserted by the fact that `mwl-syntax` and `mwl-types` gain no package-aware code path for name
  resolution at all.

## Revisiting

- **If library authors route around the rename rule** — shipping breaking changes under the same name and
  breaking consumers — the rule is not holding and either enforcement (an API-diff check at publish) or a
  ceiling mechanism becomes necessary. Watch the first ten popular packages.
- **If `mwl audit` proves insufficient in practice** and users sit on known-vulnerable versions, MVS's
  weakness is real and a narrowly scoped automatic-patch-floor mechanism should be argued — not a general
  range system.
- **If a legitimate need for a build step appears** that is not code generation a publisher could do
  beforehand, § 5 is what would have to change, and it should be re-argued from scratch rather than
  amended, because every exception in every other system began as a legitimate need.
