A counter declared in a `for` header is **function-scoped**, exactly as every other binding is. It is
readable after the loop, holding the value that ended it, and a second `for` header below it
declaring the same name again is a re-declaration diagnostic naming the first — there is no
shadowing, and the header form is given no scope of its own.

So `rule:iteration/for-init-clause` is a spelling change and not a scoping one. The declaration in a
header is the same statement a declaration anywhere else is, which is what makes the declare-once
rule, definite assignment and the lowering all apply to it unchanged.

**Memory:** none. The header declaration is the same binding in the same frame slot, with the same
lifetime, as a declaration on the line above the loop.

**Declined:**
- **A loop-scoped counter, as in C and Java.** It is a second scoping rule in a language that has
  one, and it would have to say what it means for definite assignment, for a closure that captures the
  counter, and for an `inout` argument's frame (`rule:statements/an-inout-argument-is-a-local`).
