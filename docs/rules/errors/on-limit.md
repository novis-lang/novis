`Core\Fatal::onLimit(callable(LimitReport): void $handler): void` fires only for a resource-limit
`FATAL` — memory, CPU time, `max_output`, wall time, `max_script_depth` and call-stack depth. An
internal panic never reaches it (`rule:errors/panics-bypass-user-code`).

Registration is request-local, living beside the pending-error slot in `Ctx`, and dies with the
request like every other per-request slot. The handler runs on a **reserve carved out of the
request's own budget at request start** and unavailable to ordinary execution — otherwise a request
that exhausted its memory would have nothing left to report with.

There are **two reserves, memory and time, not one per limit**, because those are the only two
resources a handler cannot run without spending; a `max_output` breach refuses the handler nothing.
Sizing them is a `System`-class decision, not the script's: a program choosing the size of its own
safety net is exactly the case where the choice should belong to someone else.

`LimitReport` is an array rather than a class. The report is built where the breach happens, in
`nvs-runtime`, which holds no `Core` class descriptor to instantiate one from — and a keyed array
takes a later field without changing the signature of a handler already written.
