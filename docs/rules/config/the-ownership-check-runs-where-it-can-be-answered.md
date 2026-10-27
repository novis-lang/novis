The ownership check runs where its question can be answered: `nvs serve` and reload, on the host and
as the account that will serve, and `nvs config check --as <account>`, which names that account.
Three readers resolve the tree without it, each for a reason of its own.

`nvs run` executes a program the invoking account named, from a working directory that account chose,
as that account: whoever can write its `./nvs.toml` can write the program, so the file adds no
authority the check would take away, and the Windows default would otherwise refuse nearly every
checkout. The data folder's `nvs.toml` read at step 3 is the same account's file, in a folder Novis
created private to it. Plain `nvs config check` and `nvs config dump` run on the auditing machine as
the auditing account, where the check answers a different question than the one it exists for — it
refuses trees the server would accept and passes trees the server would refuse, and a green result
that means neither is worse than one that does not claim to have looked.

**`nvs config check --as <account>` asks the question for the account that will serve.** It resolves
through `resolve::JudgedFor`, whose `trust` is `trust::check_as`, the function `nvs serve`'s boot runs,
so it stops at the same first file with the same `E0607` and help. On Unix the account is a user name
or a uid; on Windows a name, `LocalSystem`, or `NT SERVICE\<service>`, computed before the service
exists. A name this machine cannot resolve is `E0662`, before any file is read. `--as` judges the files
on this machine as they are now. It does not see the server's own copy of the files, a container's
read-only mount, whose owner waiver applies only where that mount is, or a deploy step that changes an
owner. The check trusts no group but the administrators one, so the account's group membership does
not change its answer. `config dump` takes no `--as`: it lists keys, and `check` is the verdict.

`nvs serve` checks every file it reads, every time it reads it: the root, each include, each secret
file and each `[[app]]` fold a request or a queue job asks for, at boot and at every reload. A boot
that finds a file another account can write stops with `E0607` naming it; a reload that finds one is
refused, names the path in the log, and the running snapshot keeps serving. Development mode is
checked too. The mode is written in the same file, so a check the mode could skip is one whoever
writes the file could skip.

A Windows checkout on a non-system drive inherits the drive's `Authenticated Users` write grant, and
on the CI runners `BUILTIN\Users`' right to create files in every folder, so the repository's own
servers fail there until both are removed. `bun nv checkout-acl` removes them once, and CI's Windows
jobs run it before they build.

Two writes are checked even where the read is not: the data folder before anything is created in it,
and the directory a project command or `nvs init` writes the shipped `nvs.toml` into.

**What the exemption may never do is grant.** A capability or a `System` directive read through an
unchecked file on the `run` path carries the invoking account's own authority and nothing more: an
`nvs.toml` can grant a CLI program nothing it could not take for itself, and the rule that places the
capability check is bound by that sentence.

**Declined:**
- **Skipping the check under `nvs serve` in development mode.** The mode comes from the checked file.
- **Naming `bun nv checkout-acl` in `E0607`'s help.** The help is read by people who run Novis, not
  by people who work on it, and its `icacls` steps already fix a checkout.
