`Core\Request`, `Core\Server` and `Core\Session` **throw** where the context is answering no inbound
request, rather than returning empty values. A spawned isolate is not a new request — it is a child
task inside the one already being handled — and the same conclusion reaches one step wider: a CLI
program, a scheduled script, a job worker and a `#[Test]` method are all running with nothing inbound.

An empty string would say the request arrived and sent nothing. Those are different facts, and
collapsing them is the silent-wrong-answer failure a program routes on. The class is `LogicError`: the
program asked a question its own situation has no answer to, and no correct program recovers from it.

A child that genuinely needs facts from the request that spawned it receives them as ordinary
arguments, deep-copied like any other value crossing the boundary
(`rule:security/isolate-values-cross-by-copy`). `Core\Env` and `Core\Cli` are not restricted this way:
environment variables and process arguments are process-wide facts already governed by the capability
and config-overlay machinery. `Core\Server::isDraining` is the same exemption inside a class that is
otherwise restricted: whether a shutdown has begun is a fact about the process rather than about a
request, so it answers in a child exactly as it answers in a CLI program, and the members of that
class that read the request are restricted with the other two.
