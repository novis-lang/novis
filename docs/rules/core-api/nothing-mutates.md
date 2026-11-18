No `Core` member mutates an argument and none takes a reference. There is no `inout $out`, no
out-parameter and no in-place variant of anything; the result is the return value.

When that is cheap is not this rule's to say: `$a = Core\Arr::sort($a)` changing `$a` in place is the
compiler's decision, and `rule:types/a-replaced-value-is-changed-in-place` says when it makes it. What a
second, by-reference spelling would buy is the aliasing rules this removes, and it would cost a second name
for one operation (`rule:core-api/one-name-one-signature`).

The cost is real and is paid at every mutation site: `$a = Arr::sort($a);` is three characters longer than
`sort($a);` and reads as a copy even though it is not one. A program that genuinely wants by-reference
argument passing has `inout` (`rule:statements/inout-is-the-by-reference-spelling`), which no `Core` member
uses.

**Declined:**
- **A by-reference mutator for the hot paths, such as `Arr::sortInPlace`.** The replacing statement is
  already changed in place wherever the compiler can prove it safe
  (`rule:types/a-replaced-value-is-changed-in-place`), so the second spelling buys only aliasing rules and
  breaks `rule:core-api/one-name-one-signature`.
