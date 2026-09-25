- **A task that dies by `rule:errors/propagation`'s return status leaves a pending message on its
  context, and whatever collects that context must not read it as a throw.** The symptom is a
  program whose deadline works printing `uncaught in a cancelled sibling: the request was cancelled`
  and then an uncaught exception at the `Core\Task::map` call site: the child's `ctx.pending()` was
  the safepoint's own record of the teardown. Ask `Ctx::cancelled()` before `pending()` (as
  `nvs_host::group::Child::run` does) and leave a cancelled child's slot empty; every future
  collector of a child context — the request boundary under `nvs serve`, whatever reports a `spawn
  script` — owes the same check. [until: gone crates/nvs-host/src/group.rs:cancelled()]
