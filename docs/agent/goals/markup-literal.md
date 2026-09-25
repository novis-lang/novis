---
milestone: M8
---
# Loop goal 44 — markup is written as a literal, not composed with an operator

``html`<span>posted by </span>{$name}` `` compiles, is a `Core\Html\Markup`, trusts its segments and
escapes its holes — so the two constructs a fragment costs today, `as Core\Html\Markup` and `+`, become
the narrow forms rather than the ordinary ones. `Core\Html::join` composes a list of fragments. A
hole-free literal is a constant-pool value that allocates nothing per execution, where the lift it
replaces allocates one object every time the line runs.

[ADR 0169](../../decisions/0169.md) decided all of it and `rule:core-classes/html-literal` states it;
this goal is the implementation, and its last stage is what flips that rule from `designed` to
`shipped`.

## Why here

After goal `finish-response` because that goal is the last one that changes what reaching the response
sink *means* — a fourth ending, and `Core\Response`'s members honouring a status a handler declared.
A literal that lowers straight into the output buffer (stage 4) is written against the sink's finished
shape; deciding its lowering against a sink that is about to grow a fourth exit is the kind of rework
this chain pays for by ordering.

Before goal `gap-zero`, for that goal's standing reason: it can only be emptied once everything that
would add to it has run.

What it needs already built: the HTML sink and the carrier (goal `core-depth`, shipped —
`crates/nvs-stdlib/src/html.rs` holds `MARKUP`, both lift symbols and `escape`), the interpolation lexer
frames and `parse_string_body` (goal `parses`), and `Core\Out::capture` answering the sink's carrier, so
a conformance case can read a rendered fragment back. Nothing after it depends on it.

## Stage 0 — the catch-up

Four sentences on disk enumerate the ways a `Markup` is obtained, and each is wrong the moment a fifth
exists. They are corrected in stage 6, not here — this stage is the list, so no session rediscovers it.
Re-grep before editing: these are line anchors, and the file moves.

- `crates/nvs-stdlib/src/html.rs:75-79` — the module doc's *"the ways to obtain one are all here"*, with
  its list of two symbols and the escape.
- `crates/nvs-stdlib/src/html.rs:199-208` — `MARKUP`'s own doc, *"§ 5 gives three ways to obtain one and
  every one of them is a language construct"*. Still true of the count's shape; the count changes.
- `crates/nvs-stdlib/src/html.rs:1555` — *"every way of obtaining a `Markup` is a language construct"*.
- `crates/nvs-stdlib/src/html.rs:1570` — the assertion message spelling the ways out, in a test.

`docs/rules/core-classes/html-auto-escape.md` already names the literal: [ADR 0169](../../decisions/0169.md)
landed that edit with the record, so it is not on this list.

## Stage 1 — the floor

Goal `finish-response`'s whole acceptance list, carried in verbatim by `tools/goal-switch.py`. Never traded.

## Stage 2 — the keystone: the literal is the string lexer's frames, not a second grammar

The one thing that makes every stage after it mechanical. `` html`…` `` opens a lexer mode that is the
double-quoted-string mode with a different delimiter and a different closer — the frame stack, the `{$`
handling and the brace-depth counting are all already there
(`crates/nvs-syntax/src/lexer.rs:1260-1264` opens a hole and `:1064-1090` counts depth), and
`parse_string_body` already turns the parts into a `Vec<StringPart>`
(`crates/nvs-syntax/src/parser/expr.rs:2505-2527`).

So the work is a delimiter, a closer token, and an AST node that carries the same `StringPart` vector
with a different type attached — **not** an HTML lexer. ADR 0169 § 4's "the compiler learns no HTML" is
the load-bearing constraint of this whole goal: if a stage finds itself tracking tags, attributes or
quoting state, it has left the design.

What is genuinely new: the `` \` `` escape and the `\{` escape in the segment scanner, and one more arm
in the unterminated-mode report (`crates/nvs-syntax/src/lexer.rs:210-232`) so an unterminated literal
says so under the existing `E0002` rather than running to end of file unnamed.

## Stage 3 — the type, and what a hole admits

`nvs check` gives the literal `Core\Html\Markup`. A hole's operand is checked exactly as an
interpolation's is — the conversion to text it already faces, `rule:security/secret-sinks-refuse` for a
`secret` value, a `tainted` one accepted — and a hole whose operand is already a `Markup` type-checks
with no conversion, because it is spliced rather than escaped.

No new diagnostic code. ADR 0169 § *Diagnostics* is the whole of it, and a session that finds itself
allocating one should re-read that section before it does: the likely cause is a check that belongs to
the sink rather than to the literal.

## Stage 4 — the lowering, and the two shapes that pay nothing

- **No holes** — fold to a constant. The `Markup` is built once and emitted into the constant pool, the
  way `rule:types/duration-literal` folds a nanosecond count, so a literal inside a loop allocates
  nothing per iteration.
- **Holes, in a sink position** — a run of writes, segment then escaped hole then segment, straight into
  the output buffer with no carrier materialised. `Lowering::lower_echo` already branches on static type
  per operand (`crates/nvs-ir/src/lower/expr.rs:577-583`), which is where the shape is recognised.
- **Holes, in value position** — one `Markup` holding the joined bytes, since the value genuinely
  escapes the sink.

`lower_markup_lift` (`crates/nvs-ir/src/lower/convert.rs:1263-1281`) is the existing `CoreCall` shape
the value-position case is a sibling of, including its ownership rows.

## Stage 5 — `Core\Html::join`

`Core\Html::join(array<Core\Html\Markup> $parts, Core\Html\Markup $separator): Core\Html\Markup`, one
row in `Core\Html`'s roster beside `escape`. It neither trusts nor escapes: every element is already a
carrier. A reference card as `rule:core-api/reference-card` requires, and the conformance case is a list
built in a `foreach` and joined, which is the shape the member exists for.

## Stage 6 — the rulebook and the four sentences

Flip `core-classes/html-literal` from `designed` to `shipped` and fill its empty `guardedBy` with the
cases this goal added, then `python tools/rules.py --render`. Correct stage 0's four sentences. Then the
generated reference, which picks up `join` on its own.

## Standing decisions

- **The compiler learns no HTML. This is the design and is not re-opened.** No tag tracking, no
  attribute model, no refusal that depends on where a hole sits in the markup. ADR 0169's
  *Alternatives rejected* argues it at length: a position-aware literal would be **stricter than
  `<?= ?>`**, which is the same hazard in the same document answered two ways, with the stricter answer
  attached to the newer syntax. If a session believes a hole is dangerous, that belief belongs to the
  sink and to *Revisiting*, not to this goal.
- **Backticks, and the `html` prefix.** Settled in § 2 of the record. If the delimiter collides with
  something the lexer already does, say so in the handoff and stop — do not substitute a quote form,
  which § 2 rejected on its own merits.
- **The hole grammar is the string's, unchanged.** Do not extend it. A hole must begin with `$`, so
  `{Money::format($c)}` is text; that is PHP's limitation, kept deliberately, and "it would be nicer if
  a static call worked" is not a reason to grow a second interpolation grammar. ADR 0169 § 3.
- **`nvs fmt`, the LSP region and `nvs convert` are rules, not code, in this goal.**
  `rule:tooling/fmt-novis-constructs`, `rule:ide/a-template-region-gets-the-editors-services-and-formatter`
  and the record's *Verification* already say what each must do, and all three tools are unbuilt —
  there is no `fmt.rs`, no `regions.rs` and no `convert.rs` to edit. Landing those sentences is what
  closes the gap; do not scaffold a tool to satisfy them, and do not open a gap-register entry for
  them either.
- **A tradeoff to state, not to weigh.** Priority 4 pays for priority 3 and for migration ergonomics: a
  new literal form is language surface, bought to remove two constructs from the most-written line in a
  web program and to make `htmlspecialchars` concatenation mechanically convertible. AGENTS.md asks a
  feature to say what it spends; ADR 0169's *Consequences* is that home. Do not stop to ask whether the
  surface is worth it.
- **The sink's blind spots are out of scope and are not a new gap.** ``html`<a href="{$url}">` ``
  accepts a `javascript:` URL, exactly as `<a href="<?= $url ?>">` does today. That is a pre-existing
  property of the sink, filed in ADR 0169's *Revisiting* against both spellings at once. Do not close it
  here, and do not record it as something this goal left behind.
