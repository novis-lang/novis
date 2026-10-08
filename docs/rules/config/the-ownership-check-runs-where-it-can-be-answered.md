The ownership check runs where its question can be answered — `nvs serve` and reload, on the host and
as the account that will serve — and three readers resolve the tree without it, each for a reason of
its own.

`nvs run` executes a program the invoking account named, from a working directory that account chose,
as that account: whoever can write its `./nvs.toml` can write the program, so the file adds no
authority the check would take away, and the Windows default would otherwise refuse nearly every
checkout. The data folder's `nvs.toml` read at step 3 is the same account's file, in a folder Novis
created private to it. `nvs config check` and `nvs config dump` run on the auditing machine as the
auditing account, where the check answers a different question than the one it exists for — it
refuses trees the server would accept and passes trees the server would refuse, and a green result
that means neither is worse than one that does not claim to have looked.

Two writes are checked even where the read is not: the data folder before anything is created in it,
and the directory a project command or `nvs init` writes the shipped `nvs.toml` into.

**What the exemption may never do is grant.** A capability or a `System` directive read through an
unchecked file on the `run` path carries the invoking account's own authority and nothing more: an
`nvs.toml` can grant a CLI program nothing it could not take for itself, and the rule that places the
capability check is bound by that sentence.
