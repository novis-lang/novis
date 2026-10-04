```nvs
Cli::live(fn($live) => {
    foreach ($files as $f) {
        $live->set([Cli\Text::plain("scanning {$f}")]);
    }
});
```

`Cli::live<T>(callable(Cli\Live): T $body): T` hands its body a `Cli\Live` with `->set(array<Cli\Text> $lines)`;
`Cli::progress<T>(uint $total, callable(Cli\Progress): T $body): T` hands it a `Cli\Progress` with
`->advance({by?: uint, label?: string})`. Both answer what the body computed, at the body's type.
**The runtime owns the cursor**: it coalesces frames on a timer rather than repainting per `set`,
diffs against the previous frame, hides and restores the cursor, and **renders nothing at all when
the stream is not a terminal**, so a piped run produces clean output instead of a smear of escape
sequences. A handle that outlives its region is dead: painting through it is a `LogicError`, because
two regions on one cursor is the state this rule exists to make unreachable.

This is the scoped-callable shape `Out::capture` and `Db::transaction`
(`rule:core-classes/db-transactions`) already use, and scoping is what makes restoration enforceable
(`rule:tooling/the-terminal-is-restored-on-every-exit-path`): a region has an end, and the runtime is
at that end on every path including a throw. Cursor primitives — `moveUp`, `clearLine`,
`alternateScreen` — are **not** the surface. They break the moment output is piped, they cannot
survive a resize, they interleave incoherently when two tasks write, and a program that dies holding
them leaves the operator's shell unusable.
