The synthesis is a single function in `nvs-types` — a sibling of `check_array_literal`, not a change to
it — returning the type an array literal would have if it were synthesized bottom-up, or nothing where
`rule:ide/the-action-answers-from-the-literal-or-not-at-all` declines. It lives in the checker's crate
rather than the editor's because putting it beside the editor would mean a second implementation of union
canonicalisation and literal widening outside the type system that owns both, and the two would drift.
The type table is the one home for what a type is; the editor asks it a question.

**No compile path calls it, and that is the more important half.** `check_array_literal` still returns
`array<mixed>` where it has no expectation. That single line is what keeps a literal checked against a
target rather than inferred (`rule:types/arrays`), keeps `var $x = [1, 2];` refused
(`rule:types/var-inference`), and keeps every no-expectation position — an `echo` argument, a bare expression
statement — typing exactly as it does today. There is no new diagnostic, no new code, nothing in any
crate but `nvs-types`, no change to the runtime array descriptor and nothing per request. A guard test
holds that no compile path reaches the function, so "this changes no language behaviour" is checkable
rather than argued.

`var` still refusing a bare array literal is not a leftover. It refuses because of where the answer ends
up, not because the answer is unavailable: this action's output is text in the file, read in a diff and
approved by a person, while a `var` binding's inferred type is visible nowhere at all. A heterogeneous
literal producing `array<int|string>` is a fact worth showing someone; the same fact attached invisibly to
a binding is how a program acquires a type nobody chose. Same computation, opposite legibility.

The function and its consumer land together: a public function with no consumer has nothing to keep it
honest.
