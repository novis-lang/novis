Every capability name that exists is on one roster, in one place, with the reasoning for what it
permits named beside it. **A name not on the roster is not a capability**, which is what makes a grant
line's unknown name checkable at boot and what makes a written query for a name nobody can grant a
compile error rather than a permanent `false`.

The roster is closed in the sense that it does not grow by accident: a new capability is one variant
and one arm in the one place a name maps to the field that grants it, which is also where a refusal
gets the spelling it prints. Two capabilities that answer different questions stay two names — being
able to read a file is not permission to run it
(`rule:security/script-spawn-capability`), and opening a connection an operator named is not opening
one a program chose.

**Reading the process environment is `env`**, the one name on the roster with no family: it is written
`env = true` under `[capabilities]` and `"env"` on a `[grants]` line, and a refusal points at
`[capabilities]`. It covers `Core\Env::get`, `secret` and `all`, and every member that returns a
deployment credential: `Core\Config::secret`, and `Core\Config::get` for a secret directive. No older
capability fits, because reading a variable opens no file, reaches no host and runs no program. With
its key not written it is granted on every kind of run, so the application reads the environment its
operator launched it with; a package holds it only through its grant line, because a deployment's
credentials live there. It is unscoped: a package grant carries no scope, and `Core\Env::all` names no
variable to ask about.

A member that reaches nothing declares that by entering the table with no capability rather than by
being absent from it (`rule:security/capability-declaration-is-one-table`), so the standard library's
own surface is a closed claim rather than a list with an exception column.
