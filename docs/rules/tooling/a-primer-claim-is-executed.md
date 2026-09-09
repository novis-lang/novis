Every claim the primer makes is proven against the compiler that ships with it: each spelling it states
as refused is fed to `nvs check` and must be refused, and each example in it is run and must print what
the primer says it prints. This is the discipline `docs/novis.md`'s examples already hold, applied to
the one document that is read by someone who has nothing else.

The primer is generated — from marked sections of the reference chapters, those chapters' front matter,
and the registry — so it cannot drift from the language, and a section that stops being true stops being
rendered rather than becoming a lie. What it carries is fixed by what an agent gets wrong without it:
the lookup protocol, one complete worked program with every shape annotated, the capability model and
the smallest `nvs.toml` that grants a file read, the refusal table, and the chapter map.

The refusal table is the highest-value part and the reason the order puts it late rather than first: a
model's prior for a language that reads like PHP is confident and wrong, so what Novis refuses and what
to write instead is worth more per byte than what Novis has. Its budget is a low four figures of tokens,
and it is met by what the primer selects — never by trimming what a selected section says.
