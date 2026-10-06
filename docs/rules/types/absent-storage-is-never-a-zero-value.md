Absent storage never reads as a zero value: a hole is answered before the program runs or thrown at.
Reading a variable that is not definitely assigned is an error at check time; `$a[]` anywhere but as
an assignment target is refused (`E0481`), so `$a[] .= "x"`, which would have to read an element that
is not there yet, does not compile; and reading an absent array key **throws**, because there is no
`null` to put in an `array<string>`, so the rule holds at run time too. A stored `null` in an
`array<?T>` is not an absent key and reads back unchanged.

`$a["k"] ?? $d` is the one exception: `??` means "absent or `null`", so the guarded read yields `$d`
rather than throwing — refusing there would refuse the one spelling written for exactly this case,
and the throw is what makes it worth writing. The guard covers every level of the chain under it, so
`$a["k"]["j"] ?? $d` yields `$d` for an absent key at either depth, and a `null` base needs no
`!= null` test in that one position. `isset` and `empty` are the same guarded read and answer rather
than throw, and so is the read `$a["k"]["j"] ??= $d` makes of its own target: an absent key at either
depth takes `$d`, and the write that follows is the plain `$a["k"]["j"] = $d`, which builds the row.
Only the read is guarded, so a nullable row under a `??=` target is refused where it is written, as it
is under `=`.
