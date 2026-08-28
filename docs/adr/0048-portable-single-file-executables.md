# ADR 0048 — A portable single-file executable appends source to the host binary; rebundling is a build-time CLI step, not a runtime one

- **Status:** Accepted
- **Date:** 2026-08-22
- **Amended by:** 0093
- **Scope:** a new `nvs-cli` subcommand that packages an entry `.nvs` file plus its statically-resolvable
  `require` graph into a single, self-contained, runnable executable — the Bun/`pkg`/`deno compile` shape —
  and the small addition to `nvs-host`'s startup path needed to run one. CLI programs only; `nvs serve` is
  explicitly out of scope (see *Decision* §1).

> **In short:** `nvs build --compile entry.nvs -o app` appends the entry file's statically-resolved source
> tree, as plain bytes with a small footer, after the host `nvs` binary's own sections — the standard
> self-extracting-executable trick, invisible to the PE/ELF loader and requiring no new runtime mechanism.
> At startup, `nvs-host` checks its own binary for that footer; if present, `require`/entry-point resolution
> reads from the embedded tree instead of argv, and every step after that — content hashing, the artifact
> cache, JIT compilation — is [ADR 0042](0042-on-disk-artifact-cache-format.md)'s existing pipeline,
> unchanged. Rebundling means an app author reruns the same build command against their own source; the
> shipped executable itself never mutates. Scope is deliberately CLI-only: a single downloaded-and-run
> executable is one trust domain, the same as any other native binary, so none of ADR 0005's root/app
> capability separation is engaged. Source ships as source, not as precompiled native artifacts — see
> *Alternatives rejected* for why that trade was made deliberately, not by default.

## Context

- The idea: let end users of an Novis CLI program get "one file, everything in it," the way `bun build
  --compile` or Go's static binaries work, and let an app author who already has that file rebuild it
  trivially when their code changes — without inventing a package format, an installer, or a second runtime.
- Nothing about `nvs run`, the artifact cache, or the CLI exists yet (M3/M6 are both ahead of M2 as of this
  writing), so this ADR is a forward design, argued against the ADRs that already fix the pieces it reuses.
- The obvious naive approaches — shipping app source in the clear, or shipping precompiled native artifacts
  instead — each have a real cost once checked against [0042](0042-on-disk-artifact-cache-format.md)'s
  actual key format. Both are examined in *Investigation* rather than asserted.
- The one decision that could have made this feature much larger than it needs to be: whether "portable
  executable" should also mean "portable HTTP server deployment." That would force an immediate answer to
  who counts as the config-owning "root" when the config travels inside an app-built artifact — engaging
  [0005](0005-config-changeability.md)'s capability-separation machinery for real. Scoped out explicitly in
  *Decision* §1.

## Investigation

- **Ship precompiled native artifacts instead of source**, using [0042](0042-on-disk-artifact-cache-format.md)'s
  own `.nvsc` file format as the payload. Rejected as the default: that ADR's cache key deliberately folds
  the whole environment — target triple, CPU feature bitset, compiler build and loaded extension set, as one
  `env_hash` — into the *address*, precisely so a wrong-environment artifact is never opened at all — a design point for a shared, revalidated disk cache,
  but the opposite of what "one portable file" wants. A single exe would need to embed one artifact per
  target/CPU-feature/compiler-version combination it wants to support, which is exactly the "one copy per
  combination" cost that ADR already names as a **Negative** consequence for a heterogeneous fleet — here
  paid up front, in the file the user downloads, rather than amortized across a server's cache directory.
  Compounding it: an artifact's `compiler_version_hash` ties it to one exact compiler build, so a bug fix in a
  later `nvs` release gives an already-shipped bundle no benefit until the app author rebuilds and
  redistributes — there's no "runtime updates itself, artifacts stay valid" path, because the artifact *is*
  part of what got shipped.
- **Ship source, unmodified.** The rejected alternative's mirror problem — cleartext app source inside the
  exe — is real but, per this ADR's *Scope*, explicitly not a requirement to solve: this feature's users are
  CLI-tool authors distributing a runnable program, the same posture as anyone shipping a Python script, a
  Node `pkg` bundle, or an Electron app's asdar archive today. None of those hide source from a determined
  reader either, and none of Novis's priority list ([0004](0004-memory-for-simplicity.md)'s ordering: security,
  correctness, latency, simplicity, memory) puts "hide the app's own source from its own user" anywhere in
  it. Chosen — see *Decision* §2.
- **Encrypting or obfuscating the payload.** Would answer the previous point's concern if it were a
  requirement. Rejected because it isn't: encryption needs a key the running binary must also hold (or fetch
  from somewhere), which protects against nothing a motivated reader of the binary itself cares about, and it
  is exactly the kind of scope this ADR's *Alternatives rejected* would otherwise be arguing Novis doesn't need.
- **A sidecar archive next to the exe, instead of appending to it.** Simpler to implement (no PE/ELF-footer
  code, no macOS signing wrinkle) but defeats the actual feature request — "one file" — turning it back into
  the two-or-more-file distributions every scripting language already has.
- **Letting the shipped executable rebundle itself** (accept new app code, re-emit a new exe from a running
  instance). Rejected: turns a build-time artifact into a mutable-executable-at-rest, reopens macOS
  code-signing on every rebundle instead of once at build time, and raises an integrity question (who is
  allowed to trigger a self-rebundle, running as what identity) with no corresponding requirement driving it.
  An app author who wants a new build already has the source; they run the build command again.

## Decision

### 1. Scope: CLI programs only, one trust domain, no new capability model

This feature ships a runnable *program*, not a deployable *service*. A `nvs build --compile`'d executable is
a single trust domain — the person who downloads and runs it is the only principal involved, the same as
running any other native binary — so none of [0005](0005-config-changeability.md)'s root-owned-`nvs.toml`/
per-app-capability-block separation applies, because there is no operator-versus-app-author boundary here to
protect. `nvs serve` is not addressed by this ADR at all: bundling a web-serving deployment this way would
mean the app's own build step controls what ships as the equivalent of a root-owned config, which is
precisely the "an application can never grant itself rights" property [0005](0005-config-changeability.md)
and the project's server-level-configuration decision exist to prevent. That is a different feature needing
its own argument, not a generalization of this one — see *Revisiting*. The same boundary reached from the
other side is [0093](0093-a-service-is-one-stored-argv-and-the-installer-is-a-sink.md) § 6: a bundle may not
install *itself* as a service either, because a privileged account executing that payload at every boot is
exactly the second principal this section's single-trust-domain argument depends on there not being.

### 2. Payload is source, not precompiled artifacts

The embedded payload is the entry file's `.nvs` source plus every file its `require` graph statically
resolves to ([0021](0021-single-file-inclusion-construct.md)), laid out as a flat list of
`(relative_path, length, bytes)` entries — no archive format, no compression (the payload is source text, not
[0042](0042-on-disk-artifact-cache-format.md)'s mmap-as-executable-pages case that compression would fight
against; source is small enough that this is not worth a new dependency). This buys back everything the
*Investigation*'s precompiled-artifact path costs: one file works on any target, needs no
per-combination duplication, and never goes stale against a newer compiler build. The cost accepted in
return: a fresh machine's first run of the bundle pays exactly the same cold JIT compile any first `nvs run`
of an uncached file already pays — not a new cost this feature introduces, just one it doesn't get to avoid.

### 3. `require` must resolve statically at build time

**This section is that rule's home.** A closed-world target has no filesystem to fall back on at runtime, so
[0021](0021-single-file-inclusion-construct.md)'s dynamic path has nothing left to resolve against: a bundled
executable's payload is exactly what got embedded, nothing more. `require` must therefore resolve entirely at
build time, and one with a dynamically computed path that cannot be resolved at bundle time fails the build
with a diagnostic naming the unresolvable expression — never a runtime fallback and never a silent omission.
`require`'s semantics do not change; only which paths are legal narrows.

Any other closed-world target inherits the rule from here rather than restating it.
[0025](0025-wasm-browser-target.md) argued the same shape for a browser tab and is retired; a bundled
executable is not contingent on anything, which is why the rule lives here now.

### 4. Startup: a footer, not a filesystem watcher or a new loader

The build command appends, after the host binary's own last section, the flat file list from §2 followed by
a fixed-size footer: `magic ("NVSB") | format_version: u16 | manifest_offset: u64 | manifest_len: u64`. This
is the standard self-extracting-executable technique — appended bytes are invisible to the PE and ELF loaders
because neither reads past the sections its own headers describe. `nvs-host`, at process start, reads its own
executable's last `sizeof(footer)` bytes; if the magic matches, it resolves the entry point and every
`require` against the embedded file list instead of the real filesystem, and hands each file's bytes to
[0042](0042-on-disk-artifact-cache-format.md)'s existing content-hash-then-cache-lookup path exactly as it
would for a file read from disk. No new cache mechanism, no new isolation boundary, no new capability: this
is the same `nvs run <entry>` code path with one different byte source for reads.

### 5. Rebundling is `nvs build --compile`, run by the app author, over their own source

`nvs build --compile entry.nvs -o app[.exe]` is a new `nvs-cli` subcommand: resolve the `require` graph per
§3, append per §4 to a copy of the current `nvs` host binary, write the result. The *shipped* executable
never mutates itself — "end users can rebundle it easily" means the app author reruns this one command, the
same way `bun build --compile` or `cargo build` are rerun, not that a running end-user copy rewrites itself.
This is also why §1's trust-domain argument holds: the only principal who ever produces the artifact is the
one who already has the source.

**macOS is the one platform wrinkle that survives every simplification above.** Appending bytes after an
already-signed Mach-O invalidates its signature. `nvs build --compile` therefore appends the payload *before*
signing and ad-hoc-signs the result by default (`codesign --sign -`), with a flag reserved for a user-supplied
identity later. Windows (PE) and Linux (ELF) have no equivalent step; the append is the entire build.

### 6. A `.nvsx` dependency bundles into the same payload with none of this ADR's problems

Tier 1 extensions ([0003](0003-extension-system.md)) are already one wasm component, portable across every
target Novis ships for. If the app's `require` graph depends on one, `nvs build --compile` embeds the `.nvsx`
file itself in the same flat file list as an opaque entry — no target-matrix problem, because a `.nvsx`
never had one.

### 7. Not a package manager

`nvs pkg` (M10) resolves dependencies into source on disk; `nvs build --compile` only packages what is
already resolved and present. The two compose (`nvs pkg install && nvs build --compile`) rather than overlap.

## Consequences

**Positive**

- No new runtime mechanism: the artifact cache, the compile pipeline, and `require` resolution are exactly
  [0042](0042-on-disk-artifact-cache-format.md)/[0021](0021-single-file-inclusion-construct.md) unchanged,
  reading from a different byte source. The only genuinely new code is the footer reader and the build
  command that writes it.
- A bundled executable works on any machine matching the host binary's own target — no per-CPU-feature or
  per-compiler-version duplication, and it never goes stale against a compiler bugfix the way an embedded
  precompiled artifact would.
- `.nvsx` extensions bundle for free, since Tier 1 was already designed to be one-binary-every-platform.
- Scoping out `nvs serve` keeps this feature from having to answer a genuinely hard question
  ([0005](0005-config-changeability.md)'s root/app separation) that nothing about "ship a CLI tool as one
  file" actually requires answering.

**Negative**

- App source is recoverable from the shipped executable by anyone who looks — accepted per *Investigation*
  as matching every comparable ecosystem's CLI-bundling story, not a regression this feature introduces.
- First run on a fresh machine pays a cold JIT compile, same as any uncached `nvs run` — there is no
  "instant on any machine" option that doesn't also reintroduce the precompiled-artifact multi-target cost
  this ADR declines to pay.
- A `require` with a dynamically computed, unresolvable-at-build-time path cannot be bundled at all. Accepted
  under § 3: a closed-world payload cannot support an open-world file lookup, regardless of which
  closed-world target it is.
- macOS needs a build-time signing step the other two platforms don't. Accepted as a one-time cost inside the
  build command, not something every rebuild's user pays.

## Alternatives rejected

- **Precompiled native artifacts as the payload.** See *Investigation* — multiplies exe size per
  target/CPU-feature/compiler-version combination and goes stale against compiler updates; the exact
  "Negative" cost [0042](0042-on-disk-artifact-cache-format.md) already names for a heterogeneous fleet, paid
  here in the shipped file instead of a shared cache directory.
- **Payload encryption/obfuscation.** See *Investigation* — answers a requirement this ADR does not have;
  the key would have to travel with the binary anyway.
- **A sidecar archive instead of appending to the executable.** Defeats "one file," the feature's own point.
- **A self-rebundling shipped executable.** See *Investigation* — turns a build artifact into a
  mutable-executable-at-rest with a repeated code-signing and integrity question, for a requirement
  ("end users can rebundle it easily") that a build-time CLI command already satisfies.
- **Extending this ADR to cover `nvs serve` bundling.** Explicitly deferred, not folded in — see *Decision*
  §1 and *Revisiting*.

## Revisiting

- **Bundling a web-serving deployment** is a different feature and needs its own ADR if pursued: it would
  have to decide who counts as the config-owning "root" when the config ships inside an app-built artifact,
  directly engaging [0005](0005-config-changeability.md)'s capability-separation machinery rather than
  sitting beside it the way this ADR does.
- **A fast-path precompiled artifact alongside the source payload** (embed one `.nvsc` for the build host's
  own target, fall back to compiling the bundled source on any mismatch) is a legitimate later optimization
  if cold-start latency on repeat runs turns out to matter for real CLI workloads — it does not need a new
  mechanism, only an additional optional entry in the same flat file list, checked against
  [0042](0042-on-disk-artifact-cache-format.md)'s existing environment fields before use.
- **Making source-hiding a hard requirement** would reopen the precompiled-artifacts-only path this ADR
  declined by default, and probably payload encryption with it — reopen only if a concrete use case demands
  it, not speculatively.

Verification, to land whenever this is built (no earlier than M6, since a warm run's whole value proposition
rests on [0042](0042-on-disk-artifact-cache-format.md)'s disk cache existing; the mechanism itself needs only
M4's usable-CLI-language baseline):

- A bundled executable runs identically to `nvs run entry.nvs` against the same source tree, on all three
  platforms.
- A `require` with a statically unresolvable path fails `nvs build --compile` with a diagnostic naming it,
  and never fails silently at run time inside the bundle.
- Deleting the on-disk artifact cache and rerunning a bundled executable reproduces byte-identical output,
  paying exactly one cold compile; a second run against a warm cache is a plain cache hit indistinguishable
  from any other `nvs run` hit.
- A macOS bundle passes Gatekeeper's ad-hoc-signature check out of the box; a Windows/Linux bundle needs no
  such step at all.
- A `.nvsx` dependency embedded in a bundle behaves identically to one loaded from `extension =` in
  `nvs.toml` for a plain `nvs run`.
