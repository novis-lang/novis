`inout` is a reserved word, written **before the type**, in every position a by-reference binding is legal:

```nvs
public static function bump(inout int $slot): int { … }

foreach ($xs as inout int $v) { … }
[inout int $a, inout int $b] = $pair;
```

It sits in the modifier slot the parser already reads ahead of the type, so `inout int $x` reads like the
`public readonly int $x` beside it and costs the grammar nothing.

The word names the mechanism. A parameter is copy-in/copy-back through a cell the call stages: the callee
gets the cell's address, and the value is copied back after the call returns — a callee that throws is the
one path that does not reach the write-back, so a caller sees no partial write. A `foreach` value binding
is the other mechanism, pushing each assignment through to the array element as it happens. Neither is an
alias.

**An `inout` parameter belongs only to a frame the call site outlives.** The copy-back is sound because
the callee's frame ends before the site's does, and two declarations break that order, so both are
refused where they are written. Calling a **generator** runs none of its body: it allocates the state
object and returns (`rule:iteration/generators`), so the staged cell is gone before the first
`advance()` while the parked frame would still address it (`E0492`). An **anonymous function** has no
call site that could stage anything: its by-value capture (`rule:types/implicit-capture`) lets it
outlive every frame in scope where it was written, so there is no frame whose end the copy-back could
be ordered against (`E0493`). A written `callable` signature (`rule:types/callable-signature`) gives a
site a parameter list to read and still no call site at which to stage a cell. Neither is a lowering
left unwritten: copying the value in would stop being a reference, which is the observable point of
`inout`. Shared mutable state is an ordinary object captured by value, and a generator takes the value
and `yield`s what its body computes from it.

An anonymous function **capturing** an enclosing `inout` parameter is a different question and is
allowed: capture is by value, so the function holds a snapshot of the cell's value taken where it is
written, and can outlive the call that staged the cell.

Languages split on this word by mechanism. Those that alias write `ref`, `&` or `var`; those that copy in
and copy back write `inout`, as Hack and Swift do. Novis copies, so it takes the second word.

**Declined:**
- **`ref`.** In the languages that spell it so, `ref` is a true alias, and Novis has no alias: `$a = &$b`
  is refused because there is nowhere to put a reference. The word describes neither of the two mechanisms
  above.
- **`&` kept at the declaration.** It is one of `&`'s five meanings, and it names the aliasing PHP does
  and Novis does not (`rule:statements/ampersand-is-not-a-by-reference-marker`).
