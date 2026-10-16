`Adder::bump(inout $n)` binds a local. `bump(inout $a["k"])` and `bump(inout $obj->n)` are refused at the
call with `E0439`, and that is the rule rather than a gap.

The mechanism is copy-in/copy-back through a cell the call stages, and an element or a property has no cell
of its own to stage. The write-back would have to re-run the whole access path after the callee returns,
against an array that may have been reallocated meanwhile, or through a property hook that would then run a
second time.

The rewrite is the mechanism written out — read into a local, pass the local, store it back — and it is
what the diagnostic's help says.
