`spawn script` runs another `.nvs` file — or a static method — as an isolated unit of work inside this
process. It shares exactly two things with its parent: the immutable compiled unit, which is
content-addressed and process-wide, and the parent's resource budget
(`rule:security/isolate-budget-is-the-trees`). Everything else is its own: arena, refcounts, values,
globals, class statics, runtime-defined constants, output buffer, include graph, autoloader state and
config overlay. Open resources are not shared and cannot be passed.

The operand is decided syntactically at the spawn site — a `string` expression, or a
`Class::method` reference written there — and nothing else. A `callable` variable is refused,
because whether *it* captures is not known statically; an anonymous function is refused with a diagnostic
naming the method form. A method entry is called with `args:`'s entries as **named arguments**, after
the copy, so `spawn script Reports::monthly with(args: ["month" => $m])` runs
`Reports::monthly(month: $m)` in the fresh isolate: the entry is an ordinary function with an ordinary
signature, typed at the boundary and callable and testable directly. A name `args:` holds that the
entry does not declare, or a required parameter it omits, is the ordinary named-argument error. A file
has no parameter list, which is why the path form reads `Core\Script::args()`
(`rule:core-classes/script-args`) instead. What crosses in either form is compiled code, so the budget,
grant and copy rules do not tell the two apart.

The isolate joins `spawn` and `spawn worker` rather than introducing a second concurrency vocabulary:
it is awaited like them, and it dies with its parent like them. It is as strong as Novis's request
boundary and no stronger — running code that must be assumed adversarial at the memory-safety level is
what the wasm component tier is for.

It exists because a script often has to run another one without being affected by it — a queue worker
running a job, a plugin, a CLI tool running user code — and an OS process is the wrong tool for that. It
costs orders of magnitude more than a task before the child has parsed a line, it inherits ambient
authority (environment, working directory, handles, the OS user's rights), the parent can only kill it
rather than account or cancel it, and its arguments cross serialised onto a command line. The isolate
reuses the machinery a request already has: the capped arena, the fresh request state, the config
overlay, the task tree and the shared compiled-unit cache, so a second isolate of the same script pays no
compilation at all.

**Every per-task value the allocator reads travels with the task.** A core interleaves requests and
isolates at every wait, and the allocator reads thread-locals, so a value left on the thread would
hold the next task to another's limits. The armed memory ceiling with its safepoint word, the sticky
refusal and the limit handler's reserve are taken off the thread when a task suspends and put back
when it resumes, and every task starts under none of them. A context is made on its creator's stack
and run on its task's: `Ctx::leave_this_stack` gives the creator back what it had armed, and
`Ctx::arm_on_this_stack` arms the context's own on the stack that runs it. The live balance is still
the thread's, and a request is charged its difference since its context was made.

- **Memory:** per in-flight isolate, one context, one pooled task stack and the values the child
  allocates, held until it ends and charged to the root's budget. Concurrent isolates each hold their
  own arena, so a tree's peak is the sum over its live isolates, under the root's cap. A finished
  isolate's arena is released wholesale, so nothing grows with the number of isolates ever spawned.
  A crossing holds one hash-map entry per distinct object in the graph being copied, for the length
  of that one copy. Each parked task keeps four words of armed state, and each switch moves them.
  Compiled code costs nothing per isolate: ten thousand isolates of one file
  compile it once.

**Declined:**
- **Running another `nvs` process.** It pays the process cost and the ambient authority above. A program
  that truly wants a separate OS process has `Core\Process` behind its own grant
  (`rule:security/process-exec-capability`).
- **Evaluating a source string.** A file has a stable identity — a cache key, a source map, something
  `nvs check` can read and an auditable granted path — and a string has none of them
  (`rule:security/no-eval`).
- **`require` with a fresh symbol table.** That isolates scope only: the heap is still shared, and with it
  refcounts, statics, output, resources and a fatal error.
- **A thread over a shared heap.** It breaks the non-atomic refcounts the thread-per-core model rests
  on, to save one copy at a boundary.
- **Running the child in the wasm sandbox.** It compiles code a second time that is already compiled
  natively, loses the shared unit cache, and buys a memory-safety guarantee Novis code has by
  construction.
- **An HTTP request to the server's own listener.** It needs a listener, an authentication story and a
  serialisation format to do what an arena boundary does in microseconds.
- **A method entry that ignores its parameter list and reads `Core\Script::args()`.**
  `Reports::monthly` is a method reference, and everywhere else a reference is called with its
  arguments, so a reader who writes `monthly(int $month)` expects the binding. An entry that reads the
  accessor cannot be called or tested as an ordinary function, and the copy is just as visible either
  way: it is the `args:` key at the spawn site.
- **An `fn` literal that captures nothing as the entry.** A closure minus its capture is a function
  without a name, and the same literal one keyword away in `spawn worker` does capture, so it would be
  one syntax with two meanings. Requiring the name costs one declaration and removes a capture-set
  checker and its diagnostic.
- **A `callable` variable as the entry, with a qualifier on the type saying it captures nothing.**
  The rule is not worth a qualifier.
- **Persistent isolates reused across requests.** Warm state is attractive, and it contradicts
  shared-nothing directly (`rule:security/no-cross-request-state`).
