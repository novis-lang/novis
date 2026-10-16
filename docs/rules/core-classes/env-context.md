`Core\Env::context(): Core\Env::Context` names the kind of run the calling code is in, with one case
for each kind of run `rule:tooling/echo-always-has-a-sink`'s table names:

| Case | The run |
|---|---|
| `Request` | an HTTP request, served or made in-process by `Core\Test::request` |
| `Command` | a CLI program: a script's main task or a `#[Command]` method; a `.nvst` program is one |
| `Isolate` | a `spawn script` child, a method entry or a limit handler's isolate |
| `Schedule` | a `[[schedule]]` fire |
| `Job` | a queued job's attempt |
| `Test` | a `#[Test]` method under `nvs test` |
| `Connection` | a connection isolate, from either `upgrade` door |

A `Core\Task` answers its parent's case, because a task is a second line of the same run. A
spawned isolate answers `Isolate` even when a request spawned it, because it has no request of its
own (`rule:security/request-state-throws-in-an-isolate`).

The reader is `Ctx::run_kind`, and two of the seven are never stored: `Request` is read from the
context's inbound request and `Connection` from its attached peer or connection event stream. So a
host that forgets to store a kind cannot make a request answer `Command`. That matters because a
program may print more on the command line than it would to a client, and the branch it takes is
this answer (Security and request isolation). The other five are a byte each constructor and host
stores once, before the program runs: `Command` from a new context, `Isolate` from every isolate
constructor, and `Schedule`, `Job` and `Test` from the host that starts that run.

The member never throws and asks no capability. Like `Core\Env::mode`, it is a fact about the run
and not an effect, and a program that had to catch an error to learn it was reading the answer from
a side effect of some other member.

`Core\Env::OS` is the same kind of fact and has the same shape: a `Core\Env::Platform` case, one of
`Windows`, `Darwin`, `Linux`, `BSD`, `Solaris` and `Unknown`. It is folded where a program names it to
the case's integer, so a comparison is an integer compare, a comparison with a string does not
compile, and a misspelled case does not compile. Printing a platform needs a `match` to its name
(`rule:enums/no-class-machinery`). The enum is `Platform` so its name differs from `OS` by more than
case.

The tradeoffs:
- **Performance:** nothing on the request path. A call is two pointer tests and one load.
- **Memory:** one byte per in-flight context, possibly none where it fits the context's padding
  (not checked). Nothing per request beyond that.
- **Usability:** a program asks one member where it caught an error, and a misspelled case is a
  compile error. A `match` over the seven cases compiles with no default arm.
- **Simplicity:** one member and two enums, and no new limit or setting. `Platform` costs nothing at
  run time.

**Declined:**
- **A boolean `isRequest()`.** It answers one of seven questions, and the next program asks
  whether it is a job.
- **Storing `Request` and `Connection` like the other five.** A host that forgot to store them
  would make a request read as a command, and the branch on that answer decides what a client
  sees.
- **A string answer.** A misspelled string compares `false` and compiles. A misspelled case does not
  compile.
