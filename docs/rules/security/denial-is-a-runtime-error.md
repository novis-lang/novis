A denied capability throws a `RuntimeError`, and a program may catch it and degrade. It is not an
escalation: a limit breach is fatal because the request has already consumed something it cannot give
back, whereas a capability denial is known *before* any work is done and leaves nothing behind. A
cache that falls back to recomputing when writing is not granted is a reasonable program, and making
the refusal uncatchable would forbid it (`rule:errors/escalation-ladder`).

No new class is added: `RuntimeError` is "the world said no", where the world is the operator. The
message names the capability **in its configuration spelling**, and for a scoped one the argument that
fell outside the grant — because the reader of that message is usually the operator, and the grant
name is the string they will add to their configuration. Its second line is the line to add, the table
it goes under and the file: ``help: add `read = ["/srv/app/data/"]` under `[capabilities.fs]` in
nvs.toml``. A file under `fs.read`, `fs.write` or `script.spawn` gives its folder; any other path, a
host, a name and an endpoint give themselves; an unscoped grant gives `key = true`. A connection
refused by the address table (`rule:security/net-address-policy`) says `cannot connect to` the address
and gives its `internal = ["10.0.0.5"]` line under `[capabilities.net]`. Every line given is one the
load check accepts, and it is on the message rather than on the rendered record, so a program that
catches the denial and logs `$e->message` logs it too.

A predicate that would otherwise answer `false` is refused rather than answered, so an ungranted
deployment never looks like a negative result.

**Declined:**
- **A dedicated `SecurityError` class.** It would let a program catch a denial specifically, but it
  costs a class in the exception tree and its cross-references, and it invites `catch (SecurityError)`
  as a way to make a policy refusal look handled. `RuntimeError` with the capability in the message is
  enough for the program that wants to degrade.
