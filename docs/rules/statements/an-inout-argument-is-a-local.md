`Adder::bump(inout $n)` binds a local, and `Adder::bump(inout $obj->n)` binds an instance property whose
class is known while compiling. Those are the two places `nvs-ir`'s `write_back_ref` can stage a reference
to. `bump(inout $a["k"])`, a static property, a property with hooks, a property named by a key, a call's
result and a literal are refused at the call with `E0439`, and that is the rule rather than a gap.

The mechanism is copy-in/copy-back through a cell the call stages, and an element has no cell of its own to
stage. The write-back would have to re-run the whole access path after the callee returns, against an array
that may have been reallocated meanwhile. A hooked property would run its hook a second time.

The rewrite is the mechanism written out — read into a local, pass the local, store it back — and it is
what the diagnostic's help says.
