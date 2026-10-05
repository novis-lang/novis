`nvs fmt` is a fixed point: reformatting its own output changes nothing, across the whole corpus. That is
what makes `--check` well-defined (`rule:tooling/fmt-check-writes-nothing`), and it holds for a range as
well as a file — the LSP's range formatting applies the identical rule set to a sub-range, a restriction on
where the rules apply and never a second rule set.

Deterministic here means **byte-stable, not source-independent**, and the difference is the whole cost of
`rule:tooling/fmt-never-reflows`. Output is a pure function of the input bytes: the same file formats to
the same result on every machine, forever. It is deliberately not a function of the parsed program. Two
files that differ only in whether their author broke a call have the same AST and still format to
different bytes, because that choice is kept. Two that broke the same list in different places converge
once `rule:tooling/fmt-a-list-is-one-line-or-one-item-per-line` holds, because a broken list has one
layout. A formatter whose output depended only on the AST
would have to choose every line break itself, which is exactly the width-fitting printer this design
declines to build. What converges is one file, run twice — never two semantically identical files.
