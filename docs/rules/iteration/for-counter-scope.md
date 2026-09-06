A counter declared in a `for` header is **function-scoped**, exactly as every other binding is. It is
readable after the loop, holding the value that ended it, and a second `for` header below it
declaring the same name again is a re-declaration diagnostic naming the first — there is no
shadowing, and the header form is given no scope of its own.

So `rule:iteration/for-init-clause` is a spelling change and not a scoping one. The declaration in a
header is the same statement a declaration anywhere else is, which is what makes the declare-once
rule, definite assignment and the lowering all apply to it unchanged.
