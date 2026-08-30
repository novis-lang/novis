# ADR 0123 — Spreadsheet reading and generation are one sandboxed component with no I/O

- **Status:** Accepted
- **Date:** 2026-08-30
- **Scope:** where spreadsheet reading and generation are placed — Tier 1, first-party, the roster
  amendment that admits a fourth entry — and the contract any implementation must meet: the no-I/O rule,
  what reading may never execute, formulas as typed values, the macro refusal, the read/write format
  split, explicit second-wave evaluation, the declared-size cap, and byte-reproducible output. Not in
  scope: the API of the builder and the component's entry points, which the milestone that builds it
  designs the way [0120](0120-the-image-component-is-a-pipeline-that-crosses-the-boundary-once.md) did
  for images; the sandbox contract itself ([0003](0003-extension-system.md)); how the package is named,
  pinned and granted ([0081](0081-packages-are-digests-resolution-is-a-maximum.md)); how an upload
  reaches the builder ([0105](0105-an-uploaded-file-is-a-stream-and-there-is-one-way-to-receive-it.md));
  CSV, which stays `Core\Csv`'s ([0051](0051-standard-library-tiers.md) § 3); scheduling — no milestone
  owns this, deliberately.
- **Depends on:** [0003](0003-extension-system.md) — the sandbox and the cost model that shape the
  boundary; [0051](0051-standard-library-tiers.md) — the tests that decide the placement.
- **Amends:** [0051](0051-standard-library-tiers.md) § 3 — the first-party Ext roster gains a fourth
  component, unscheduled; the whole-roster sentence and the Consequences bullet now read four.
  [0121](0121-pdf-generation-is-sandboxed-html-rendering-with-no-io.md) § 1 and its first Consequences
  bullet — the restated roster count becomes a pointer to 0051 § 3, the count's one home.

> **In short:** Novis will own one spelling for spreadsheet files, as a first-party Tier 1 `.nvsx` —
> package `nvs/spreadsheet`, namespace `Novis\Spreadsheet` — fourth on
> [0051](0051-standard-library-tiers.md)'s roster and, like the PDF renderer, **unscheduled**: this ADR
> fixes placement and contract so the eventual milestone designs an API, not a policy. The component has
> **no I/O** — a workbook crosses as bytes and comes back as bytes, and an external reference in a read
> file is data, never a fetch. Reading **executes nothing**: a formula returns as text plus the file's
> cached result, VBA is inert payload. Writing closes the genre's injection class by type: only an
> explicit `Formula` value becomes a formula cell, so a `tainted` string beginning `=` is a text cell by
> construction. Macros are never written — a filled `.xlsm` template emits `.xlsx`. CSV stays
> `Core\Csv`, one spelling per job. The boundary is crossed the way
> [0120](0120-the-image-component-is-a-pipeline-that-crosses-the-boundary-once.md) crosses it — a
> Novis-side builder, whole sheets and whole documents per call, no per-cell accessor — and the output is
> byte-reproducible: no timestamps, no generator entropy. Unlike PDF, the engines exist, pure Rust and
> wasm-clean — `calamine` reads xlsx/xlsm/xlsb/xls/ods, `rust_xlsxwriter` writes xlsx to PhpSpreadsheet's
> feature depth, `umya-spreadsheet` fills styled templates in place, IronCalc (MIT/Apache-2.0, pre-1.0)
> evaluates when its wave comes — so the cost is the unifying model and builder: a milestone's work, but
> a small one, which is why placement and contract are decided now and scheduling is not.

## Context

- **What PHP demonstrates here is not [0121](0121-pdf-generation-is-sandboxed-html-rendering-with-no-io.md)'s
  fragmentation.** Userland converged on one dominant library, PhpSpreadsheet (née PHPExcel), so the
  "five incompatible subsets" half of the PDF argument is absent and this ADR does not borrow it. What
  the category carries instead: **hostile bytes** — xlsx is a zip of XML, where XXE and decompression
  bombs are the genre's CVE history, and legacy xls is an OLE compound binary — and an **injection class
  on the way out**: a cell whose text begins `=`, `+`, `-` or `@` executes in the spreadsheet
  application that opens the export, and every userland library documents that as the caller's problem.
- **The audience produces and consumes these files constantly.** The export button is as universal in
  [0080](0080-the-audience-nvs-is-built-for.md)'s applications as the invoice that motivated 0121 — the
  two generators ship together in practice — and the import path is an uploaded workbook,
  attacker-controlled by definition.
- **[0051](0051-standard-library-tiers.md) § 2's tests, applied in order.** Test 1: no — a workbook is
  read or built within one request, and nothing outlives it. Test 2: no, *because* § 3 below types the
  formula — a writer that cannot be tricked into emitting code needs no launderer, which is exactly what
  keeps it out of Core. Test 3: no, by § 2's design. Test 4: no — parsing or serializing a document
  costs milliseconds, so the boundary crossing is noise. Test 5: **emphatically** — zip, XML, OLE CFB
  and embedded images make an uploaded workbook the tier's exact material. Test 6: yes, and it cuts both
  ways: one spreadsheet spelling, and no second CSV API, because `Core\Csv` already holds that job.
- **The engines exist — the inverse of 0121's finding.** `calamine` reads every format that matters,
  lazily per sheet; `rust_xlsxwriter` writes xlsx with charts, conditional formats, tables, autofilters
  and images; `umya-spreadsheet` edits an existing styled workbook in place; `spreadsheet-ods` writes
  ods; IronCalc evaluates formulas, MIT/Apache-2.0 and pre-1.0. All pure Rust, all wasm-clean, no C leg
  at all. The missing piece is **one document model** over crates whose types share nothing, plus the
  Novis builder — a milestone's work, and far smaller than the pagination layer that sized the PDF
  engine.

## Decision

### 1. Placement: Ext, first-party, fourth on the roster, unscheduled

Spreadsheet reading and generation are a **Tier 1 `.nvsx`**, first-party, distributed as the package
**`nvs/spreadsheet`** with everything a program names under **`Novis\Spreadsheet`** — the two-payload
package shape and the one-registered-class rule follow
[0120](0120-the-image-component-is-a-pipeline-that-crosses-the-boundary-once.md) § 1, with
**`Novis\Spreadsheet\Engine`** as the manifest's one class and every other name Novis source that calls
it. It is not Core: a PhpSpreadsheet-sized API is priority 4's permanent cost, and its parsers are test
5's material — unsandboxed in every in-flight request is the outcome
[0051](0051-standard-library-tiers.md) exists to prevent. It is not Native: nothing in it holds
connection state or privilege. 0051 § 3's first-party Ext roster gains this entry **unscheduled**: no
milestone owns it, M9 still builds two, and this ADR exists so that when the work is scheduled the
placement and the contract are already decided rather than improvised.

### 2. The component has no I/O, and reading executes nothing

A workbook crosses the boundary as bytes and the result returns as bytes; the component fetches
nothing. An external-workbook reference, a linked image or a remote data connection in a read file is
returned as **data** — the reference itself — and never resolved; an image placed on write is bytes the
caller supplies. This is [0121](0121-pdf-generation-is-sandboxed-html-rendering-with-no-io.md) § 2's
rule without the asset map, because a workbook is self-contained.

Reading evaluates nothing. A cell reads as its stored scalar, or as formula text plus the cached result
the file's author wrote beside it — the two distinguishable at the API, because a cached value is a
claim by the author, not a computation by us. VBA and embedded OLE objects are inert payload the reader
may enumerate but can never execute. Qualifier contagion is
[0055](0055-extension-qualifier-declarations.md) § 1's: cells read from `tainted` bytes are `tainted`.

### 3. A formula is a typed value, never a string that starts with `=`

Only an explicit `Formula` value produces a formula cell. Every string writes as a text cell —
including one beginning `=`, `+`, `-`, `@` or a control character, the trigger set for the
formula-injection class — so an export built from user data is inert by construction rather than by a
caller remembering to prefix a quote. This is
[0024](0024-taint-tracking-for-injection-sinks.md)'s move — code and data separated by type, not by
inspection — applied at a boundary PHP userland leaves as a footnote.

### 4. The boundary is crossed the way 0120 crosses it, and a bomb is refused before allocation

The builder assembles the whole document — sheets, cells, styles, charts — as Novis values, and one
call serializes it. Reading opens a workbook once and pulls whole sheets or declared ranges as bulk
blocks. **There is no per-cell accessor, and none is added later** —
[0120](0120-the-image-component-is-a-pipeline-that-crosses-the-boundary-once.md) § 3's refusal: a
member that would be called in a loop over the file's own contents costs the boundary, not the work.

Before any sheet buffer is allocated, the component reads the declared dimensions and the container's
declared uncompressed sizes, and refuses a workbook over a cell cap with a throw naming the cap and the
declared size — the cap is policy and the sandbox's memory cap the backstop, the shape 0120 § 6 gave
`max_pixels`. The knob, its default and its config block are the building milestone's to argue.

### 5. The roster is read-wide, write-narrow, and a macro is never written

| Format | Read | Write |
|---|---|---|
| xlsx | yes | yes — the parity target: styles, merged cells, charts, conditional formats, tables, autofilters, images, defined names |
| xlsm | yes — data only; VBA enumerable, inert | **never** — a filled template emits xlsx with no VBA stream |
| xlsb | yes | no |
| xls (BIFF8) | yes | no — a 1997 binary writer is legacy nothing should produce |
| ods | yes | yes, second wave |

Template fill is named because it is the workflow behind most real exports: open an existing styled
workbook, set values, save — styling preserved, macros dropped. Implementation: `calamine` for reading,
`rust_xlsxwriter` for generation, `umya-spreadsheet` for template fill, `spreadsheet-ods` for the
second wave — every crate pure Rust and on [deny.toml](../../deny.toml)'s allowlist, so
[0051](0051-standard-library-tiers.md) § 4's questions never arise.

### 6. Evaluation is explicit, second-wave, and reports what it cannot compute

Nothing evaluates implicitly — not on read (§ 2), and not on write, where a `Formula` cell is written
for the opening application to compute. An explicit evaluate call is the **second wave**, over IronCalc
— the one credible Rust engine, whose pre-1.0 state is what gates the wave, a version test rather than
a judgment. Its function set is documented, and the result carries the list of functions and references
it could not compute, so a test asserts the list is empty —
[0121](0121-pdf-generation-is-sandboxed-html-rendering-with-no-io.md) § 4's visibility rule applied to
formulas: silent wrongness becomes a named, testable diagnostic.

### 7. Output is inert and byte-reproducible

The writer emits no VBA, no embedded executables and no launch-shaped content; hyperlinks exist only
where the document wrote them. It embeds **no timestamps and no generator entropy** — document
properties carry fixed dates unless the caller sets them, archive entries a fixed time — so equal input
gives byte-equal output and a document test is a `Core\Test` byte comparison, the rule 0121 § 5 set for
PDFs.

**What it spends**, per request that calls the component: the document model and its serialized form,
inside the extension's memory cap, none of it outliving the request. A million-row export belongs in
`Core\Queue`, and the write path must offer a bounded-memory streaming mode — `rust_xlsxwriter`'s
constant-memory mode is the existence proof — so a large export costs rows in flight, not rows total.

## Consequences

- **[0051](0051-standard-library-tiers.md)'s roster consequence reads four**: image and intl scheduled
  for M9, the PDF renderer and this component unscheduled, M9's scope untouched. The admission is argued
  honestly: not on fragmentation, which PhpSpreadsheet disproves, but on a priority-1 read surface plus
  a write-side injection default that only protects as *the* spelling — at an assembly cost the existing
  engines keep small.
- **The genre's two vulnerability classes split cleanly**: hostile workbook bytes are contained by the
  sandbox and refused early by § 4's cap; formula injection is absent by construction, not mitigated.
- **A cost this ADR does not hide:** PhpSpreadsheet's surface is wide and parity is a moving judgment.
  The building milestone owes the concept-mapping table
  [0120](0120-the-image-component-is-a-pipeline-that-crosses-the-boundary-once.md) § 11 is the precedent
  for, so "what happened to X" has one home when the API exists.
- **A cost this ADR accepts:** until § 6's wave, Novis returns a workbook's cached formula results where
  PhpSpreadsheet would recalculate. The two are distinguishable at the API, so the gap is visible rather
  than silent.

## Alternatives rejected

- **Core.** A PhpSpreadsheet-sized permanent `Core` API (priority 4) with zip, XML and OLE parsers
  unsandboxed in every in-flight request (priority 1) — both halves of what 0051 refuses at once.
- **Native (Tier 2).** No connection lifetime, no privilege; the sandbox is the feature, not a cost to
  route around.
- **Written in Novis.** Memory-safe, and PhpSpreadsheet proves the shape possible — but it reimplements
  OOXML and BIFF lore that production crates already carry, unfuzzed and ours to maintain, to buy
  nothing the wasm build does not give. Novis code belongs in the builder, the line
  [0121](0121-pdf-generation-is-sandboxed-html-rendering-with-no-io.md) already drew.
- **Leave the category to the third-party channel.** The honest competitor, and stronger here than for
  PDF, since PHP shows one userland library can carry it. Rejected because the read surface is test 5's
  material and belongs where the sandbox and the cap are guaranteed present; because § 3's default only
  protects as the spelling everyone holds; and because the engines' existence makes first-party assembly
  cheap enough that declining it saves almost nothing.
- **Expose the crates' surfaces directly.** Three disjoint APIs for one job — test 6, inside a single
  component.
- **Evaluate formulas on read.** PhpSpreadsheet's implicit-recalculation habit turns opening an
  attacker's upload into executing the attacker's program. §§ 2 and 6 refuse it twice.

## Revisiting

IronCalc at 1.0 unlocks § 6's wave. A unified pure-Rust read-write crate reaching maturity replaces the
assembly wholesale and this ADR is not reopened: §§ 2–7 are written against the interface, not the
crates, and a swapped engine must still meet them.

## Verification

Owed by the milestone that eventually builds the component, recorded here so scheduling inherits them:

- A read and a write with **no capability grants** succeed, and no network or filesystem access is
  observable from the guest.
- An xlsx whose sheet XML declares an external entity, and one carrying an external-workbook reference,
  each read with the reference surfaced as data and nothing resolved (§ 2).
- A workbook whose declared dimensions or declared uncompressed size exceed the cap throws before a
  sheet buffer is allocated, and the same bytes pass when the cap is raised (§ 4).
- A `tainted` string beginning `=` lands as a text cell, and the same expression as a `Formula` value
  lands as a formula cell — asserted on the written bytes, not the builder's state (§ 3).
- A filled `.xlsm` template emits output in which a scanner finds no `vbaProject` stream (§ 5).
- The same input serialized twice is byte-equal (§ 7).
- [0051](0051-standard-library-tiers.md) § 5's existing test already covers that the component cannot
  register a `Core\` name.
