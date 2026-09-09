---
milestone: M8
---
# Loop goal 25 — `Core\Compress`, `Core\Mime` and `Core\Zip`

Spec § 17's three archive-and-encoding classes, each Tier 0 for the same reason: what a compressed or
archived input can do to a server is **policy**, and policy must be non-optional. When this goal is
green a program can decompress under a bound it cannot switch off, detect a type by magic bytes rather
than by a rule interpreter, and read an archive whose `../` entries, absolute paths and symlink entries
are refused before anything touches a filesystem.

**They are M8's, not M9's.** `rule:core-api/tier-roster` puts all three at
Tier 0 (`0051:96-100`), and [m9.md](../../plan/m9.md) is the extension system — `.nvsx` loading, the WIT
world, the capability bridge — and carries none of them.
`crates/nvs-stdlib/tests/spec-classes-part-two-outstanding.txt` said M9 carried the § 17 four; that
comment was wrong and this goal is half of what corrects it, `Core\Xml` being the other half in goal `xml-tree`.

**It sits after goal `net-os-signal`** because `Core\Mime`'s detection is what a `Core\Net` or `Core\Http` reader
hands bytes to, and before goal `xml-tree` because `Core\Xml` is the largest of the four and shares nothing
with these three.

## Stage 0 — the catch-up

1. **The § 17 rows** in `docs/spec/01-core-library.md` are one line each and two of them carry no ADR.
   The surface is settled in this goal's stages and those rows are edited to match.
2. **`Core\Response`'s `Content-Encoding` handling.** `rule:core-api/tier-roster` says the built-in server compresses
   nothing itself, per `rule:http-server/two-deployments-and-nothing-a-proxy-owns`. That
   stays true: this goal gives a *program* a compressor and does not put one in the server's path.
   Any comment implying the server will grow one is corrected here.

## Stage 1 — the floor

Goal `net-os-signal`'s whole acceptance list, carried in verbatim by `tools/goal-switch.py`. Never traded.

## Stage 2 — `Core\Compress`, and the bound that cannot be switched off

1. **Four codecs, one API** — gzip, deflate, brotli, zstd, replacing `gzopen` handles, `deflate_init`
   contexts and `zlib.*` stream filters. The codec is an enum, never a string: there is no
   `compress($data, "gzip")` spelling for the same reason there is no cipher-name-as-string in
   `Core\Crypto`.
2. **A decompression bound is a parameter with a default, not an option that can be `null`.** The
   ratio and the absolute output ceiling are both bounded; exceeding either throws rather than
   truncating, because a truncated decompression that looks like success is the bug this class exists
   to prevent. `[limits]` supplies the default and a call may lower it, never raise it past the
   configured ceiling.
3. **Dependencies**, picked under [ADR 0051 § 4](../../decisions/0051.md) — pure-Rust
   implementations for all four, each owing a notice regeneration
   (`python tools/gen-attribution.py`). Nothing here takes the audited-C exception.
4. **Streaming and whole-buffer are one surface or explicitly two**, decided in the module doc: the
   `Core\Xml` precedent is that a tree and a stream are different jobs stated as such, and the same
   sentence has to be written here or explicitly not apply.

## Stage 3 — `Core\Mime`, magic bytes and no rule interpreter

1. **Detection by magic bytes**, per the § 17 row — a fixed table this crate carries, not libmagic's
   rule language and not a file extension. A type detected from an extension is a type an attacker
   chose.
2. **The answer is a closed enum plus an "unknown", never a free string**, so a caller cannot compare
   against a spelling that never occurs.
3. **A detected type is not a laundering.** Bytes that detect as `image/png` are still `tainted`;
   detection answers what something *looks* like and nothing about whether it is safe. This is the one
   sentence most likely to be got wrong by a caller, so it is in the member's own doc card.

## Stage 4 — `Core\Zip`, where the policy lives

1. **Three refusals, before any byte reaches a path**: a `../` or otherwise traversing entry, an
   absolute-path entry, and a symlink entry. Refused at *read* time by the reader, so a program cannot
   opt out by extracting entries itself.
2. **A decompression bomb is stage 2's bound, applied per entry and across the archive** — the
   ratio and ceiling are the same rule, and `Core\Zip` does not get a second one.
3. **Extraction takes a destination and never escapes it**, checked after path resolution rather than
   before, so a symlink that appears during extraction cannot win the race.
4. **The proofs are the attacks**: an archive with a traversing entry, one with an absolute entry, one
   with a symlink entry, and one bomb — each refused by the diagnostic that names the rule, not by a
   failed file operation.

## Stage 5 — the three keys are struck

`spec-classes-part-two-outstanding.txt` loses `§17 Core\Compress`, `§17 Core\Mime` and `§17 Core\Zip`;
`Core\Xml` stays for goal `xml-tree`. The migration table's `zlib`, `fileinfo` and `zip` rows answer a `Core`
spelling.

## Standing decisions

- **This goal may open one ADR number**, for the decompression bound shared by `Core\Compress` and
  `Core\Zip` — it is one rule with two callers and would otherwise be stated twice. The three classes'
  surfaces are `rule:core-api/tier-roster` rows and get no numbers of their own.
- **No bound is optional and none can be raised past the configured ceiling.** A call may ask for
  less. There is no spelling for "unbounded", in the same way `Core\Http\Options` has no spelling for
  an unbounded wait.
- **A refusal is a diagnostic naming the rule, never a failed I/O error**, so a caller cannot confuse
  "this archive is hostile" with "this disk is full".
- **The server still compresses nothing** (`rule:http-server/two-deployments-and-nothing-a-proxy-owns`
  ). A program may compress its own response body; nothing in the response path does it implicitly.
- **Ambiguity about surface resolves toward whole-buffer first**, with streaming added only where the
  migration table names a PHP shape that cannot be expressed without it — recorded in the module doc,
  never `BLOCKED`.
- **What this spends**, per `rule:programs/memory-priority`: the output buffer,
  bounded by the ceiling above and attributable to the request that asked for it. A streaming reader
  holds one window and not the document.
