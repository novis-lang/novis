Every claim the primer makes is proven against the compiler that ships with it: each example in it is
run and must print what the primer says it prints, and each diagnostic code it names must be one that
compiler declares. `bun nv reference --primer --check` is that proof, and it is the harness
`docs/novis.md`'s examples already run under, applied to the one document that is read by someone who
has nothing else.

A code is checked as a code because the text beside it is often a fragment that no `nvs check` can be
handed. The `E0xxx` is the executable half: it either names a constant in the diagnostic registry or
it names nothing, and a code the compiler cannot raise is the one lie a document generated from marked
sections can still tell.

The primer is generated — from marked sections of the reference chapters, those chapters' front matter,
and the registry — so it cannot drift from the language, and a section that stops being true stops being
rendered rather than becoming a lie. What it carries is fixed by what an agent gets wrong without it:
the lookup protocol, one complete worked program with every shape annotated, the capability model and
the smallest `nvs.toml` that grants a file read, and the chapter map. It carries no table of PHP
spellings and their replacements: `rule:programs/no-compatibility-promise` allows no such table, and an
agent that writes a PHP habit meets the diagnostic for it at its first `nvs check`.

Its budget is a low four figures of tokens, and it is met by what the primer selects — never by
trimming what a selected section says.
