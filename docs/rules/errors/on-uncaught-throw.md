`Core\Fatal::onUncaughtThrow(callable(Throwable): void $handler): void` fires when an ordinary
`THROWN` propagates through every frame uncaught and reaches the isolate or request root.

It needs no reserve of its own: execution was healthy until this point, so the request's ordinary
remaining budget applies. It receives the **real `Throwable` object**, not copied data — this is
the root itself, not a boundary something has to copy across.

The zero-retry rule of `rule:errors/escalation-ladder` applies unchanged: a handler that itself
faults drops straight to `rule:errors/handler-script`.

After this tier and before native teardown, `Core\Script::onExit`'s queue runs. It is not a tier of
the ladder — it runs at every non-fatal ending including successful ones, needs no reserve, and
observes the ending rather than reporting a failure. A handler faulting here changes nothing about
it.
