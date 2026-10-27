A run of `&&`, `||` or `??` has two layouts: on one line, or one operand per line with the operator at
the start of the line. It is broken when a line break its author wrote sits between two of its
operands, on either side of an operator. `rule:tooling/fmt-every-binary-operator-breaks-operator-first`
gives the same layout to every other binary operator but `.`: a broken `.` chain keeps its author's
lines, with each `.` at the start of its line.

A broken chain keeps its first operand where it was and starts each following operand on a line one
level in, operator first. When a run mixes operators of different precedence, the chain is the run at
the lowest precedence, and each operand made of higher-precedence operators is judged on its own.

The condition of an `if`, `elseif`, `while` or `do … while` takes one more step when it is broken: the
`(` ends its line, each operand of the condition's top-level chain starts a line one level in, and the
`)` starts a line of its own, followed by ` {` where a block follows. A line break after the `(`,
between two operands of that top-level chain, or before the `)` breaks the condition. A break inside
a call's arguments belongs to that call. Every control header breaks as a condition does, and a break
inside a higher-precedence group of the condition breaks it too
(`rule:tooling/fmt-every-control-header-breaks-like-a-condition`).

```nvs
if ($user->isActive() &&
    $user->hasRole('admin') && $request->isSecure()) {
    Audit::log($user);
}

if (
    $user->isActive()
    && $user->hasRole('admin')
    && $request->isSecure()
) {
    Audit::log($user);
}
```

The first statement formats to the second. A condition written on one line stays on one line, and
an unbroken chain anywhere else does too.
