# ADR 0087 — An unterminated bidirectional control is rejected at every boundary, by one predicate

- **Status:** Accepted
- **Date:** 2026-08-24
- **Scope:** the rule that a bidirectional embedding, override or isolate control must be terminated inside
  the span that opened it; the one predicate that decides it; and its three callers — the lexer, the
  terminal sink and the HTML escaper. Also, explicitly, what is *not* covered: zero-width and invisible
  characters, homoglyphs, and the full Unicode Bidirectional Algorithm, each with its reason in § 4.
- **Amends:** [0024](0024-taint-tracking-for-injection-sinks.md) § 5 — `Core\Html::escape` neutralizes an
  unterminated control alongside the four characters it already escapes; the auto-escape sink therefore
  covers this class without a second call.
  [0086](0086-core-cli-terminal-is-a-sink.md) § 1 — the terminal sink's substitution table gains the same
  rule, and § 7's refusal to address bidi is **removed**: it rested on the claim that neutralizing would
  refuse valid Arabic and Hebrew, which is true of a blanket ban and false of the balanced rule below.
  [docs/implementation-plan.md](../implementation-plan.md) — the lexer check is buildable now; the two sink
  halves land with the milestones that build those sinks.
- **Amended by:** none.
- **Relates to:** 0009, 0029, 0030, 0049, 0056, 0062, 0080

> **In short:** Trojan Source ([CVE-2021-42574](https://trojansource.codes/)) makes text render in an order
> its bytes do not have, so a reviewer approves one program and the compiler builds another. MWL is already
> immune to two thirds of that paper: **identifiers are ASCII-only**
> ([`lexer.rs`](../../crates/mwl-syntax/src/lexer.rs)), which removes homoglyph identifiers and bidi
> identifiers structurally, without a lint. What remains is comments, string literals, inline-HTML text and
> runtime data. The attack condition in all four is the same and it is narrow: **a directional control that
> opens a scope and is never closed before its span ends.** Balanced controls — what legitimate Arabic,
> Hebrew and mixed-direction text actually uses — are untouched, which is what makes rejecting the rest
> affordable. So there is **one predicate** and three callers: the lexer makes it a **hard compile error
> with no suppression**, matching [ADR 0029](0029-identifier-casing-is-checked.md)'s stance on every other
> spelling question; `Core\Cli`'s sink and `Core\Html::escape` **neutralize** it, substituting `�` exactly
> as [0086](0086-core-cli-terminal-is-a-sink.md) § 1 already does for a C1 code point.

## Context

- **Trojan Source is a rendering attack, not a parsing one.** The bytes say one thing and the terminal,
  editor or diff view says another, so code review — the control this project's contribution model depends
  on — is the thing being defeated. The paper's proof-of-concept exploits live in **comments and string
  literals**, which is why "our identifiers are ASCII" is not on its own an answer.
- **Two of the three classes are already closed, and it is worth being exact about which.**
  [`crates/mwl-syntax/src/lexer.rs`](../../crates/mwl-syntax/src/lexer.rs) admits `_` and
  `is_ascii_alphanumeric()` and nothing else, so a Cyrillic `а` cannot appear in an identifier and neither
  can U+202E. Rust needs `mixed_script_confusables` and `text_direction_codepoint_in_literal` as two
  separate lints; MWL needs only the second, and this ADR is it.
- **The industry answer is already settled and is worth matching rather than re-deriving.** Rust 1.56.1
  shipped `text_direction_codepoint_in_literal`/`_in_comment` **deny-by-default**; GCC 12 shipped
  `-Wbidi-chars`, on by default for the unterminated case. Both draw the line at *unterminated*, not at
  *present*, because banning the characters outright breaks real internationalized text.
- **The reason this is one ADR and not three paragraphs.** The predicate is identical at the lexer, at the
  terminal and at the HTML escaper. Deciding it separately in each place is how the same rule acquires three
  slightly different definitions — the failure this repository's "state a fact once" rule exists to prevent.
  It is also why the decision is taken now, while only one of the three callers is buildable: the other two
  otherwise get argued again, from scratch, by whoever writes them.
- **An incoherent posture would be worse than none.** A language whose headline is compile-time qualifiers
  ([0080](0080-the-audience-mwl-is-built-for.md)) cannot guard its terminal output against display
  deception and leave its web output open, when the escaper is a `Core` member either way.

## Decision

### 1. The predicate

Nine code points open or close a directional scope. Two more are marks, not scopes, and are **not** covered
because they open nothing and therefore cannot be unterminated:

| Code points | Role | Closed by |
|---|---|---|
| U+202A `LRE`, U+202B `RLE`, U+202D `LRO`, U+202E `RLO` | embedding / override — the legacy forms, and the ones every published exploit uses | U+202C `PDF` |
| U+2066 `LRI`, U+2067 `RLI`, U+2068 `FSI` | isolate — the modern forms Unicode recommends | U+2069 `PDI` |
| U+200E `LRM`, U+200F `RLM` | mark — affects one character's resolved direction, opens no scope | *not covered* |

**A span is rejected when it ends with a scope still open.** Two independent counters, one for embeddings
and overrides and one for isolates: an opener increments, `PDF` and `PDI` respectively decrement with a
floor of zero, and both must be zero at the end of the span.

This is deliberately **simpler than [UAX #9](https://www.unicode.org/reports/tr9/)**, whose directional
status stack has interactions between the two kinds that this counter model flattens. That is correct here
because the question is not *how does this render* — which needs the real algorithm — but *does this span
leave a scope open*, which the counters answer soundly. The simplification can only over-approximate
towards rejecting, never towards accepting, which is the safe direction for a security check.

### 2. The span, per caller

| Caller | Span | On failure | Lands |
|---|---|---|---|
| `mwl-syntax`'s lexer | one token — a string literal, a comment, or an inline-HTML run — **and additionally each line inside a multi-line one**, so a heredoc cannot hide a scope across its lines | **hard compile error**, no suppression | buildable now |
| `Core\Cli`'s output sink ([0086](0086-core-cli-terminal-is-a-sink.md) § 1) | one `echo` operand or one `Cli::write` call | each unmatched control becomes `�` (U+FFFD) | M8 |
| `Core\Html::escape` ([0024](0024-taint-tracking-for-injection-sinks.md) § 5) | one escape call | the same substitution | M7 |

**The lexer is an error and the sinks substitute**, and the asymmetry is the same one
[0086](0086-core-cli-terminal-is-a-sink.md) § 1 already draws. Source is written by a developer who can fix
it, and a build that fails names the file and line; runtime data arrives from outside, where refusing to
print it would turn a display concern into an availability one. Substituting `�` rather than deleting keeps
[0086](0086-core-cli-terminal-is-a-sink.md) § 1's property that a neutralized byte is *visible* — the reader
sees that something was removed instead of silently reading a shorter string.

**No suppression at the lexer**, per [ADR 0029](0029-identifier-casing-is-checked.md) § 1's standing
position: this project fails a build over a lowercase class name, and an attribute that switches off a
security check would be the first exception. A file that legitimately needs an unterminated control does not
exist — terminating it is always available and always correct.

### 3. What this costs

Nothing measurable. The lexer already walks every byte of every token; the check is two counters carried
through a walk that happens anyway. Both sinks are already single-pass scans
([0086](0086-core-cli-terminal-is-a-sink.md) § 1), and this adds two counters to each. No allocation, no
table lookup — the eleven code points are a match arm.

### 4. What is deliberately not covered

- **Zero-width and invisible characters** — U+200B `ZWSP`, U+200C `ZWNJ`, U+200D `ZWJ`, U+2060 `WJ`,
  U+FEFF. Rejected as a target because the false-positive cost is real and the attack value is now low:
  `ZWNJ` is **required** in Persian, `ZWJ` in Indic scripts and in every multi-codepoint emoji sequence, and
  with identifiers ASCII-only these characters can no longer make two names look alike. They can pad a
  string, which is a deception with no mechanism behind it.
- **Homoglyphs** — closed structurally by the ASCII identifier rule. Nothing to add, and a confusables
  table ([UTS #39](https://www.unicode.org/reports/tr39/)) would be dead weight.
- **The full Bidirectional Algorithm**, per § 1 — MWL does not render text and has no business
  implementing UAX #9.
- **`Core\Log`** — deliberately, and for the reason
  [0024](0024-taint-tracking-for-injection-sinks.md) § 4 already gives for `tainted`: the writer is a
  JSON-Lines serializer, which escapes these code points as `‮` by construction, and recording exactly
  what an attacker sent is the point of a security log.

## Consequences

- **A PHP file containing an unterminated control does not convert.** `mwl convert` (M11) reports it as a
  source error rather than carrying it across, which is the correct outcome and is worth stating so it is
  not filed as a converter bug.
- **One diagnostic, one implementation, three callers.** The predicate lives in `mwl-syntax` beside the
  lexer that needs it first, and both `Core` sinks call it rather than reimplementing it. If a future
  session finds itself writing a second copy, that is the bug.
- **MWL is fully closed against the Trojan Source paper** once all three callers exist — identifiers by
  construction, everything else by this rule. That is a claim worth being able to make plainly, and
  [0080](0080-the-audience-mwl-is-built-for.md)'s audience is exactly the one that asks.
- **A cost this ADR accepts:** an editor or diff viewer that does not itself neutralize these controls will
  still misrender an MWL file *before* the compiler ever sees it. The check catches it at build time, not
  at review time, so it protects the merge and not the reading. Closing that properly is `mwl-lsp`'s
  ([0016](0016-ide-integration.md)) and is not scheduled here.

## Alternatives rejected

- **Ban the eleven code points outright, terminated or not.** Breaks legitimate mixed-direction string
  literals — an Arabic error message interpolating a Latin identifier needs an isolate to render correctly
  — and would push developers to build such strings by concatenation to evade the check, which is worse
  than the disease.
- **A warning rather than an error at the lexer.** A warning nobody fails a build on is not a control, and
  [ADR 0029](0029-identifier-casing-is-checked.md) already settled that this project does not have a
  suppression story for spelling rules.
- **Handle it only in `mwl fmt`.** [ADR 0039](0039-canonical-code-formatting.md) fixes that `fmt` is never
  wired into `mwl check`, so a formatter-only rule is one an attacker's PR simply does not run.
- **Handle it only at the sinks, not in source.** Inverts the severity: the source case is the one with a
  CVE and a working exploit against code review, and the runtime cases are display deception.
- **Implement UAX #9 properly**, per § 1.
- **Also reject invisibles**, per § 4.

## Verification

- The predicate's own unit tests: balanced `LRE…PDF` and `RLI…PDI` accepted; each of the seven openers
  unterminated, rejected; a stray `PDF`/`PDI` with no opener accepted, since it closes nothing and opens
  nothing; `LRM`/`RLM` accepted anywhere.
- A `.mwlt` case per source span — a comment, a single-quoted literal, a double-quoted literal with
  interpolation, a heredoc whose scope opens on one line and closes on the next (**rejected**, per § 2's
  per-line rule), and an inline-HTML run — each asserting a compile error naming this ADR.
- A `.mwlt` case with a genuinely balanced Arabic string literal, asserting it **compiles and round-trips
  unchanged**. This is the case that fails if someone later "simplifies" the rule to a blanket ban.
- The Trojan Source paper's own `commenting-out` and `stretched-string` patterns, transliterated to MWL, as
  named fixtures.
- At the sinks (M7/M8): a fixture writing an unterminated `RLO` asserts `�` in the frozen output, and one
  writing balanced Arabic asserts the text survives byte-for-byte.
