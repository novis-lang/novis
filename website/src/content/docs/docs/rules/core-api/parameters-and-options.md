---
# GENERATED FILE — written by website/scripts/sync-rules.mjs from docs/rules/. Do not edit.
title: "Parameters and the options bag"
description: "Optional knobs are one trailing shape literal — and required, optional and nullable stay three separate questions."
editUrl: false
lastUpdated: false
tableOfContents: false
prev:
  link: /docs/rules/core-api/one-way-to-do-each-thing/
  label: "One way to do each thing"
next:
  link: /docs/rules/core-api/shape-parameters/
  label: "Shape parameters"
---

<p class="nv-section-lead">Optional knobs are one trailing shape literal — and required, optional and nullable stay three separate questions.</p>

<div class="nv-counts"><div class="nv-count" data-kind="total"><span class="nv-count-value">9</span><span class="nv-count-label">rules</span></div><div class="nv-count" data-kind="shipped"><span class="nv-count-value">8</span><span class="nv-count-label">shipped</span></div><div class="nv-count" data-kind="designed"><span class="nv-count-value">1</span><span class="nv-count-label">designed</span></div><div class="nv-count" data-kind="php"><span class="nv-count-value">5</span><span class="nv-count-label">differ from PHP</span></div></div>

<ol class="nv-rule-list"><li><a href="#callback-receives-value-and-key">A callback always receives <code>($value, $key)</code>, and may declare fewer parameters than the call site passes</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#parameters-are-callable-by-name">Every <code>Core</code> parameter is callable by the name the spec writes, and that name is compatibility surface</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#options-bag">A member's optional knobs are one trailing shape literal, never a flag or a bitmask</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#required-optional-and-nullable">Required, optional and nullable are three separate questions: nullability belongs to the type and optionality to the default</a><span class="nv-rule-list-status" data-status="designed">Designed</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#omission-is-not-a-written-null">An options bag tells an omitted key from a written <code>null</code></a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#a-nullable-field-omits-as-the-never-written-marker">A bag or shape field is nullable exactly when its omission fills the never-written marker</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#a-written-null-removes">Wherever a <code>Core</code> member admits a written <code>null</code> it means remove, and <code>&quot;&quot;</code> is never a removal spelling</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#the-marker-never-reaches-a-program">The never-written marker is not in the type system and is never handed to user code</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#the-bag-abi-is-unchanged">The three-state bag changes which constant fills a slot and nothing else about the ABI</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li></ol>

<div class="nv-rule" id="callback-receives-value-and-key">

## A callback always receives `($value, $key)`, and may declare fewer parameters than the call site passes

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#callback-receives-value-and-key"><code>core-api/callback-receives-value-and-key</code></a>
</div>

A callback a `Core` member invokes always receives `($value, $key)`, in that order, and a closure may
declare fewer parameters than the call site passes ([`types/callable-arity`](/docs/rules/types/closures/#callable-arity "A closure satisfies a callable type when its arity is at most the type's, matched from the left")). A closure wanting only
the value writes one parameter and never sees the key.

This kills the whole `ARRAY_FILTER_USE_KEY`/`ARRAY_FILTER_USE_BOTH` flag family, which exists in PHP only
because its callbacks have a fixed arity, and it removes the need for `map`/`mapWithKey` pairs that would
otherwise violate [`core-api/one-name-one-signature`](/docs/rules/core-api/naming-and-shape/#one-name-one-signature "One name has one signature, and two behaviours need two names"). The order is value-first because that is the
argument almost every callback uses, so the common closure is `fn($v)` with nothing to skip.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p><code>ARRAY_FILTER_USE_KEY</code> and <code>ARRAY_FILTER_USE_BOTH</code> have nothing to select, and there is no <code>map</code>/<code>mapWithKey</code> pair</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-api/one-way-to-do-each-thing/#no-mode-strings" title="A mode is an enum, never a string or an integer constant, and only four grammars are exempt"><code>core-api/no-mode-strings</code></a> <a href="/docs/rules/types/closures/#callable-arity" title="A closure satisfies a callable type when its arity is at most the type's, matched from the left"><code>types/callable-arity</code></a> <a href="/docs/rules/types/closures/#callable-is-a-closure" title="callable is satisfied by a closure and by nothing else, and no object is callable"><code>types/callable-is-a-closure</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0063.md">record 0063</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/arr-map-and-filter.nvst"><code>tests/conformance/core/arr-map-and-filter.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/arr-map-keys-rekeys-and-group-by-partitions.nvst"><code>tests/conformance/core/arr-map-keys-rekeys-and-group-by-partitions.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="parameters-are-callable-by-name">

## Every `Core` parameter is callable by the name the spec writes, and that name is compatibility surface

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#parameters-are-callable-by-name"><code>core-api/parameters-are-callable-by-name</code></a>
</div>

Every parameter of every `Core` member may be written by name at the call site — the `$name` the spec's
signature column writes, and the trailing bag under the one name `options` — under exactly the rules a
user-declared method's parameters follow ([`types/arrays`](/docs/rules/types/arrays-and-property-keys/#arrays "An array is PHP's ordered hash with string keys and a declared element type")): written order is evaluation order, a name
fills its own slot, a defaulted parameter may be skipped, a positional after a name is refused, and a name
never reaches a variadic tail.

A parameter's **name is compatibility surface**, versioned where its type is, so renaming one is a
breaking change. That is the price of the feature and it is paid deliberately: the spec has published
every parameter name since it was written, so the surface was already public, and a surface a user's own
method has that a `Core` member lacks is one more rule to learn. The name lives once, on the registry row
beside the type, and the reference card looks it up rather than repeating it
([`core-api/reference-card`](/docs/rules/core-api/lifetimes-and-absences/#reference-card "An implemented Core member carries its reference card in the registry declaration beside its code")). Nothing on the request path changes — a `name:` resolves while checking
and the helper ABI is untouched.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-api/parameters-and-options/#options-bag" title="A member's optional knobs are one trailing shape literal, never a flag or a bitmask"><code>core-api/options-bag</code></a> <a href="/docs/rules/core-api/lifetimes-and-absences/#reference-card" title="An implemented Core member carries its reference card in the registry declaration beside its code"><code>core-api/reference-card</code></a> <a href="/docs/rules/types/arrays-and-property-keys/#arrays" title="An array is PHP's ordered hash with string keys and a declared element type"><code>types/arrays</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0063.md">record 0063</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0117.md">record 0117</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/reject/a-misspelled-name-at-a-core-member-is-unknown.nvst"><code>tests/conformance/reject/a-misspelled-name-at-a-core-member-is-unknown.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-stdlib/tests/spec_registry_coverage.rs"><code>crates/nvs-stdlib/tests/spec_registry_coverage.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-stdlib/src/registry.rs"><code>crates/nvs-stdlib/src/registry.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="options-bag">

## A member's optional knobs are one trailing shape literal, never a flag or a bitmask

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#options-bag"><code>core-api/options-bag</code></a>
</div>

After the subject come the required arguments in dataflow order, and then **at most one** trailing optional
shape literal — an object literal ([`types/object-top`](/docs/rules/types/objects-and-shapes/#object-top "object is the opaque top of every class type")) declared as a `type` alias. There are no `bool`
flag parameters, no `int` bitmasks, and no positional optional tail longer than one.

The bag is the home of every optional knob because it is named, order-free and structurally checked, and a
bag written entirely from compile-time constants folds to a constant. It also flattens at the call site
into one argument per declared field ([`core-api/shape-flattens-at-the-abi`](/docs/rules/core-api/shape-parameters/#shape-flattens-at-the-abi "A shape argument flattens into one argument per field of the merged list, so no shape exists at run time")), so nothing is allocated
to carry it. A bag field may be nullable, and where it is, leaving the key out and writing `null` into it
are two different requests ([`core-api/omission-is-not-a-written-null`](/docs/rules/core-api/parameters-and-options/#omission-is-not-a-written-null "An options bag tells an omitted key from a written null")).

The cost is that a bag's keys are declared and fixed: a member that must take a key whose *name* is chosen
at run time needs a second member taking a `string`, which is why `Core\Uri` carries both `with` and a
query-parameter pair rather than one member doing both.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>There are no <code>bool</code> flag parameters, no <code>int</code> bitmask constants and no positional optional tail longer than one</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-api/parameters-and-options/#parameters-are-callable-by-name" title="Every Core parameter is callable by the name the spec writes, and that name is compatibility surface"><code>core-api/parameters-are-callable-by-name</code></a> <a href="/docs/rules/core-api/shape-parameters/#shape-parameter" title="A fixed-key shape parameter is one type carrying its arms, and it is only ever a whole parameter"><code>core-api/shape-parameter</code></a> <a href="/docs/rules/core-api/parameters-and-options/#omission-is-not-a-written-null" title="An options bag tells an omitted key from a written null"><code>core-api/omission-is-not-a-written-null</code></a> <a href="/docs/rules/types/objects-and-shapes/#object-top" title="object is the opaque top of every class type"><code>types/object-top</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0063.md">record 0063</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0147.md">record 0147</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0135.md">record 0135</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-types/tests/core_members.rs"><code>crates/nvs-types/tests/core_members.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-codegen/tests/core_str.rs"><code>crates/nvs-codegen/tests/core_str.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/command-run-fills-an-unwritten-option-from-its-declared-default.nvst"><code>tests/conformance/core/command-run-fills-an-unwritten-option-from-its-declared-default.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="required-optional-and-nullable">

## Required, optional and nullable are three separate questions: nullability belongs to the type and optionality to the default

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#required-optional-and-nullable"><code>core-api/required-optional-and-nullable</code></a>
</div>

"Does `?T` mean the key may be absent, or that `null` is a legal value?" has a better answer than choosing:
they are separate questions, and both already have a spelling.

| Declaration | Key absent | Key present as `null` |
|---|---|---|
| `T $x` | an issue: required field missing | an issue: `null` is not permitted |
| `?T $x` | an issue: required field missing | accepted, decodes to `null` |
| `T $x = <default>` | accepted, the default is used | an issue: `null` is not permitted |
| `?T $x = null` | accepted, `null` is used | accepted, decodes to `null` |

Nullability is a property of the **type** and optionality is a property of the constructor parameter's
**default**; neither borrows the other's meaning. Encoding is the plain inverse: every field is always
emitted, including a `null` one. There is no omit-when-null option, because an asymmetric encoder is a
round-trip bug that only shows up in the value that happens to be absent — and adding one later, conditioned
on the field having a default, is purely additive.

**One row is designed, not shipped.** `?T $x = null` needs a written `= null` parameter default, which
the checker still refuses — `crates/nvs-types/src/defaults.rs`'s own gap, not this table's. The other
three are answered at every door that reads a derived codec, a row's columns included: the default is
evaluated while compiling into a constant that rides on the field itself
(`nvs_runtime::CodecField::default`), so a decoder fills an absent optional key without being the call
site that would otherwise emit one. A constructor position **no field names** — a property
`skip: true` removed from the contract, which [`core-classes/derive-field-list`](/docs/rules/core-classes/codecs-sessions-and-signatures/#derive-field-list "The field list is the declared property list, and every field is a constructor parameter of the same name") sanctions — has no
field to carry a constant and is filled by nothing, so the call that asks for an instance out of a
document is refused while compiling rather than answered with a guess: `E0820` at a document door and
`E0806` at a row's.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p><code>?T</code> never means &quot;the key may be absent&quot;, and there is no omit-when-null encoding option</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-api/everything-is-written/#written-participation" title="A class participates in a wire format only where it writes so, never structurally"><code>core-api/written-participation</code></a> <a href="/docs/rules/core-api/parameters-and-options/#omission-is-not-a-written-null" title="An options bag tells an omitted key from a written null"><code>core-api/omission-is-not-a-written-null</code></a> <a href="/docs/rules/core-api/parameters-and-options/#the-marker-never-reaches-a-program" title="The never-written marker is not in the type system and is never handed to user code"><code>core-api/the-marker-never-reaches-a-program</code></a> <a href="/docs/rules/types/unions-and-conversion/#unions-and-mixed" title="A union permits only what every member permits, and mixed is the one position checked nowhere"><code>types/unions-and-mixed</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0071.md">record 0071</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0063.md">record 0063</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/json-decode-as-admits-exactly-its-declared-type.nvst"><code>tests/conformance/core/json-decode-as-admits-exactly-its-declared-type.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/json-decode-as-fills-an-absent-optional-key-from-its-default.nvst"><code>tests/conformance/core/json-decode-as-fills-an-absent-optional-key-from-its-default.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/json-derive-decodes-and-reports-every-field.nvst"><code>tests/conformance/core/json-derive-decodes-and-reports-every-field.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-stdlib/src/json.rs"><code>crates/nvs-stdlib/src/json.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="omission-is-not-a-written-null">

## An options bag tells an omitted key from a written `null`

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#omission-is-not-a-written-null"><code>core-api/omission-is-not-a-written-null</code></a>
</div>

A call site's two ways of not giving an options-bag field a value are two different requests. `{}` leaves a
component alone; `{fragment: null}` removes it ([`core-api/a-written-null-removes`](/docs/rules/core-api/parameters-and-options/#a-written-null-removes "Wherever a Core member admits a written null it means remove, and  is never a removal spelling")). Before this, an
omitted field arrived at the helper as a null and a written null arrived as the same null, so a field that
could itself hold one was unusable and every bag field had to be non-nullable.

The mechanism is a constant rather than a sentinel: an omitted **nullable** field materializes a
never-written marker instead of a null ([`core-api/a-nullable-field-omits-as-the-never-written-marker`](/docs/rules/core-api/parameters-and-options/#a-nullable-field-omits-as-the-never-written-marker "A bag or shape field is nullable exactly when its omission fills the never-written marker")),
so the helper reads three states out of one argument and the flattening is untouched
([`core-api/the-bag-abi-is-unchanged`](/docs/rules/core-api/parameters-and-options/#the-bag-abi-is-unchanged "The three-state bag changes which constant fills a slot and nothing else about the ABI")). An in-band marker — `""`, a magic string, a reserved constant —
is a value the field's own type admits, so user data can arrive as one by accident; that is the bug class
this removes rather than relocates.

`crates/nvs-stdlib/src/registry.rs`'s `Const::NeverWritten` is the declaration and its two guards,
`a_nullable_option_omits_as_the_never_written_marker` and
`a_nullable_shape_field_omits_as_the_never_written_marker`, hold the pairing over every registered row.
`Core\Uri::with` is the first member to spend it.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>PHP has no way to ask either question of an argument; here the two are different requests at every <code>Core</code> member</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-api/parameters-and-options/#a-nullable-field-omits-as-the-never-written-marker" title="A bag or shape field is nullable exactly when its omission fills the never-written marker"><code>core-api/a-nullable-field-omits-as-the-never-written-marker</code></a> <a href="/docs/rules/core-api/parameters-and-options/#a-written-null-removes" title="Wherever a Core member admits a written null it means remove, and  is never a removal spelling"><code>core-api/a-written-null-removes</code></a> <a href="/docs/rules/core-api/parameters-and-options/#the-bag-abi-is-unchanged" title="The three-state bag changes which constant fills a slot and nothing else about the ABI"><code>core-api/the-bag-abi-is-unchanged</code></a> <a href="/docs/rules/core-api/parameters-and-options/#options-bag" title="A member's optional knobs are one trailing shape literal, never a flag or a bitmask"><code>core-api/options-bag</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0147.md">record 0147</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0063.md">record 0063</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0135.md">record 0135</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0022.md">record 0022</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-stdlib/src/registry.rs"><code>crates/nvs-stdlib/src/registry.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/uri-with-tells-an-omitted-component-from-a-written-null.nvst"><code>tests/conformance/core/uri-with-tells-an-omitted-component-from-a-written-null.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="a-nullable-field-omits-as-the-never-written-marker">

## A bag or shape field is nullable exactly when its omission fills the never-written marker

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#a-nullable-field-omits-as-the-never-written-marker"><code>core-api/a-nullable-field-omits-as-the-never-written-marker</code></a>
</div>

A field of an options bag or of a shape arm carries a default, and that default is what an omitting call
site materializes. Three declarations, three behaviours:

| Declaration | Key absent | Key present as `null` |
|---|---|---|
| no default | a compile error: required key missing | a compile error: `null` is not of the field's type |
| a default, non-nullable type | the default is materialized | a compile error: `null` is not of the field's type |
| a default, **nullable** type | the never-written marker is materialized | `null` is materialized |

Only the third row is new, and the rule that holds it is a **pairing**: a field admitting `null` — a
nullable, a union with a null arm, or a `mixed`, which admits one without spelling it — is admitted exactly
when its default is the never-written marker, and a field not admitting `null` is admitted exactly when its
default is a null or a literal. A registry row getting the pairing wrong fails the build.

The pairing is what makes "filled" readable at all: the constant standing for *omitted* has to be one no
written value can also be, or the two states collapse again.

The two guards in `crates/nvs-stdlib/src/registry.rs` —
`a_nullable_option_omits_as_the_never_written_marker` and
`a_nullable_shape_field_omits_as_the_never_written_marker` — assert exactly this pairing, one per
spelling, over every registered row.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-api/parameters-and-options/#omission-is-not-a-written-null" title="An options bag tells an omitted key from a written null"><code>core-api/omission-is-not-a-written-null</code></a> <a href="/docs/rules/core-api/parameters-and-options/#the-marker-never-reaches-a-program" title="The never-written marker is not in the type system and is never handed to user code"><code>core-api/the-marker-never-reaches-a-program</code></a> <a href="/docs/rules/core-api/shape-parameters/#shape-flattens-at-the-abi" title="A shape argument flattens into one argument per field of the merged list, so no shape exists at run time"><code>core-api/shape-flattens-at-the-abi</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0147.md">record 0147</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0135.md">record 0135</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0022.md">record 0022</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-stdlib/src/registry.rs"><code>crates/nvs-stdlib/src/registry.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/reject/a-uri-component-with-no-removal-refuses-a-written-null.nvst"><code>tests/conformance/reject/a-uri-component-with-no-removal-refuses-a-written-null.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="a-written-null-removes">

## Wherever a `Core` member admits a written `null` it means remove, and `""` is never a removal spelling

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#a-written-null-removes"><code>core-api/a-written-null-removes</code></a>
</div>

Wherever a `Core` member admits a written `null` — a bag field or an ordinary argument — it means
**remove**. Clear, delete, not present in the result. It never means "restore a default", never selects an
alternative behaviour, and is never a flag under another name. An **omitted** key means the receiver's
existing value is carried through unchanged.

The position the `null` arrives in is incidental to what it means, which is why the rule is stated over a
member rather than over a bag field. A query builder that drops a pair whose value is `null` already
behaved this way before the rule existed — the only behaviour that round-trips, since a query string cannot
spell an absent value — and that it was arrived at independently is the best evidence it is right.

The rule's teeth are that **an input is made nullable only where removal is something the member can
actually do.** A member with no removal to offer keeps its non-nullable field, and writing `null` into it
stays the compile error it is today, so the nullability in a signature *is* the announcement that the thing
can be cleared and a caller reads it off the type rather than off prose. `""` is never a removal spelling
on any member: it is a legal value of most of these fields and is already distinguishable — an empty query
is not an absent one — so overloading it would reinstate the in-band sentinel this removes.

`crates/nvs-stdlib/src/uri.rs`'s `removable` helper is the first reader of the three states, and
`Core\Uri::with` the first member to spend them: `{port: null}`, `{query: null}` and
`{fragment: null}` each clear their component, where omitting the key carries it over.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>PHP spreads clearing across <code>unset</code>, <code>&quot;&quot;</code>, <code>false</code> and per-function sentinels; here one spelling means one thing at every member</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-api/parameters-and-options/#omission-is-not-a-written-null" title="An options bag tells an omitted key from a written null"><code>core-api/omission-is-not-a-written-null</code></a> <a href="/docs/rules/core-api/lifetimes-and-absences/#a-lifetime-is-written" title="A signature's lifetime is a required key and null is the forever spelling"><code>core-api/a-lifetime-is-written</code></a> <a href="/docs/rules/core-api/one-way-to-do-each-thing/#no-mode-strings" title="A mode is an enum, never a string or an integer constant, and only four grammars are exempt"><code>core-api/no-mode-strings</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0147.md">record 0147</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0063.md">record 0063</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-stdlib/src/uri.rs"><code>crates/nvs-stdlib/src/uri.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/uri-with-tells-an-omitted-component-from-a-written-null.nvst"><code>tests/conformance/core/uri-with-tells-an-omitted-component-from-a-written-null.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="the-marker-never-reaches-a-program">

## The never-written marker is not in the type system and is never handed to user code

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#the-marker-never-reaches-a-program"><code>core-api/the-marker-never-reaches-a-program</code></a>
</div>

The never-written marker is not a value a program can hold, observe or name. It is **not in the type
system** — a nullable field's declared type does not contain it, no expression evaluates to one, and the
call site that "writes" it writes nothing at all while the compiler materializes the fill. It is **never
handed to user code**: its only reader is the native helper behind the member, which turns it into the
member's own behaviour before anything returns, and a helper that fails to handle it is a contract
violation in this repository's own code rather than a wrong answer given to a program. And it is
**transient**: it exists for the duration of the call that materialized it, is consumed by the helper, and
is never stored, returned or reachable from a value a program holds.

A **user-declared** function's optional parameter is unchanged, deliberately. `?int $x = null` in program
source still cannot tell an omitted argument from a written `null`, because closing that would need a
spelling for asking the question — an `isset` on a parameter, or a third state a program can observe — and
that is a language-surface decision this one does not open. A native helper can be held to reading three
states by a test; a user's function body cannot be, and giving it a state it has no way to name would be
exactly the observable marker this rule refuses.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-api/parameters-and-options/#a-nullable-field-omits-as-the-never-written-marker" title="A bag or shape field is nullable exactly when its omission fills the never-written marker"><code>core-api/a-nullable-field-omits-as-the-never-written-marker</code></a> <a href="/docs/rules/core-api/parameters-and-options/#the-bag-abi-is-unchanged" title="The three-state bag changes which constant fills a slot and nothing else about the ABI"><code>core-api/the-bag-abi-is-unchanged</code></a> <a href="/docs/rules/core-api/parameters-and-options/#required-optional-and-nullable" title="Required, optional and nullable are three separate questions: nullability belongs to the type and optionality to the default"><code>core-api/required-optional-and-nullable</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0147.md">record 0147</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0022.md">record 0022</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-runtime/src/value.rs"><code>crates/nvs-runtime/src/value.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="the-bag-abi-is-unchanged">

## The three-state bag changes which constant fills a slot and nothing else about the ABI

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#the-bag-abi-is-unchanged"><code>core-api/the-bag-abi-is-unchanged</code></a>
</div>

The three-state bag changes *which constant fills one slot, for one kind of field*, and nothing else. A bag
is still one ABI argument per declared field, no runtime representation of a shape exists, and no helper
learns a new calling convention ([`core-api/shape-flattens-at-the-abi`](/docs/rules/core-api/shape-parameters/#shape-flattens-at-the-abi "A shape argument flattens into one argument per field of the merged list, so no shape exists at run time")).

The marker reused is the never-written storage state that already exists for definite-initialization
analysis: already defined as distinct from every legal value including null, already costing **zero
additional bytes** because it is one more discriminant on a representation that carries one, and already
non-refcounted, so it raises no ownership question at a call boundary. Reaching it from a call site costs
one constant-argument variant, one instruction constant beside the null one, and the codegen arm that
writes the tag byte.

What it spends ([`programs/memory-priority`](/docs/rules/programs/claims-and-priorities/#memory-priority "Memory buys security, correctness, latency and simplicity — bounded, attributable and stated")): **nothing per request and nothing per call** — the
omitting call site emits one constant either way. A non-nullable field is unaffected in every respect, so
every member registered today lowers to the same instructions and nothing needs migrating.

`crates/nvs-runtime/src/value.rs` carries the marker tag, `nvs_ir::ir::InstKind::ConstUnset` is the
one instruction that materializes it, and `nvs_codegen`'s arm for that writes the tag byte over a
zero payload. Every member registered before this lowers byte for byte as it did.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-api/shape-parameters/#shape-flattens-at-the-abi" title="A shape argument flattens into one argument per field of the merged list, so no shape exists at run time"><code>core-api/shape-flattens-at-the-abi</code></a> <a href="/docs/rules/core-api/parameters-and-options/#a-nullable-field-omits-as-the-never-written-marker" title="A bag or shape field is nullable exactly when its omission fills the never-written marker"><code>core-api/a-nullable-field-omits-as-the-never-written-marker</code></a> <a href="/docs/rules/programs/claims-and-priorities/#memory-priority" title="Memory buys security, correctness, latency and simplicity — bounded, attributable and stated"><code>programs/memory-priority</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0147.md">record 0147</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0135.md">record 0135</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0022.md">record 0022</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-runtime/src/value.rs"><code>crates/nvs-runtime/src/value.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-ir/src/ir.rs"><code>crates/nvs-ir/src/ir.rs</code></a></dd></div></dl>

</div>
