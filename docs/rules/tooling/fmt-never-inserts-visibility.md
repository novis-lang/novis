`rule:core-api/written-visibility` makes an omitted visibility keyword a compile error. Two tools
meet that error, and they answer it in opposite ways.

**`nvs fmt` never inserts the keyword.** The formatter orders modifiers and does not supply a missing
one. A formatter that inserted `public` would make a file's *meaning* depend on whether a tool had
been run over it, and would restore PHP's implicit default through the back door for anyone who
formats on save. A file that does not compile still does not compile after `nvs fmt`.

**`nvs convert` does insert it**, as an **E-tier** row of its rule table
(`rule:tooling/convert-one-table-two-modes`): PHP's omission provably means `public`, so writing the
word is a behaviour-identical rewrite, discharged by a differential case like any other E branch
(`rule:tooling/convert-equivalent-is-proven`). Porting a PHP file therefore costs the author nothing
here, and the ported member reports the level PHP actually gave it.
