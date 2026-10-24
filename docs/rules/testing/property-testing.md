`#[Property]` runs a method against generated inputs, and the generators are **derived from the
parameters' declared types** by the same declaration walk a codec is derived from, user classes
included. That is why this belongs to the compiler rather than to a package: only a compiler can
walk a declared type, and a userland version needs a hand-written generator per type, which is the
boilerplate that keeps property testing rare.

A failure shrinks to a minimal counterexample and reports the seed that reproduces it exactly. A
`gen:` option narrows one parameter where its declared type is wider than the domain. A generated
`string` is valid UTF-8 including combining marks and grapheme clusters, because that is what a
`string` is — a property that only holds for ASCII is a property that is not true.

**The attribute.** `#[Core\Test\Property]` goes on a method with a `#[Test]`'s shape: `public`, not
`static`, `: void`, with a body. Its options are `#[Test]`'s less `retries:` and `because:`, plus
`runs:` (1 to 100000, 100 by default) and `gen:`. It is test code
(`rule:testing/tests-never-reach-a-build`). **Every parameter is generated**: a `#[TestWith]` row, a
second marker, and a fixture are never asked. Its rows ride in their own table, beside the `#[Test]`
one (`crates/nvs-types/src/property.rs`).

**What has a generator.** Every scalar, `decimal`, `bytes`, a `string` and either `tainted` form, an
enum and a single case, a single-value type, `mixed`, `?T`, `array<T>`, an inline shape, a union, and a
concrete, non-generic class of the program with a `public` constructor, built from its constructor's
parameters. A class may reach itself through `?T`, a list or a union. Anything else is `E0890` at the
method: a function, an interface, an abstract or `Core` class, a class that contains itself with
nothing between, and a `secret` value, because a failing input is printed.

**`gen:`** is `{<parameter>: "<low>..<high>"}`, both ends inclusive. A number's range bounds the
value; a `string`'s, `bytes`' or `array<T>`'s bounds the length, in grapheme clusters for a string, up
to 10000. A `?T` parameter narrows its `T`. An entry naming no parameter, a range that does not parse,
and a range on a type with no range are `E0890`.

**The values.** A SplitMix64 stream seeded from the run's seed and the call's index, and nothing
else, so one seed gives one sequence of inputs. Call `i` of `n` has a size rising from 0 to 100, which
caps every length's share of its range, and a number takes one of its range's edges about one call in
eight. A float is finite. A string's pieces are ASCII, control characters, code points of two, three
and four bytes, and clusters of several code points: combining marks, U+200D emoji sequences, flags,
Hangul jamo and a Devanagari conjunct (`crates/nvs-types/src/property/generate.rs`).

- **Memory:** one call's inputs hold at most 2^20 elements, bytes and string pieces together, per
  property in flight. An input whose shortest form is over that is refused before it is built.

**Not on disk.** The runner does not call a property yet, and nothing shrinks a counterexample.

**Declined:**
- **`retries:` on a property.** A failure is reproduced from its seed and shrunk, so a retry would
  only hide the input that broke it.
- **A fixture filling a property's parameter.** A parameter is generated or it is not, and a fixture's
  value cannot be shrunk.
- **NaN and the infinities by default.** Most numeric code is not written for them, so every property
  over a `float` would first have to exclude them.
- **A `gen:` that names a generator function.** A range is checked when the program compiles and says
  the domain where it is written; a function is a second place to read.
