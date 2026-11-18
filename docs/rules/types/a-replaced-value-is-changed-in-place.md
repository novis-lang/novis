A statement that replaces a value with one made from it changes that value in place whenever no
other holder can see the old one, and the compiler decides that, never the program's author.
`$a = Core\Arr::slice($a, 1)`, `$this->items = $this->addItem($this->items, $x)`, `$s = $s . "x"`,
`$list = [...$list, $x]` and `$a[$k] = Core\Str::trim($a[$k])` each write into the value they read,
and copy only where two values are really both observed afterwards.

**One pass decides every move and every lend.** The model is reference counting with reuse, the
technique of Koka's Perceus and Lean 4: a value whose count is 1 at the point of change is reused, and
the compiler makes the count 1 wherever the program allows it. The lowering emits plain retains and
releases and decides nothing. One IR pass, `crates/nvs-ir/src/ownership.rs`, runs after it and decides
from liveness and from what each instruction may do. Every syntactic special case the lowering had
for this is folded into the pass and deleted, so there is one place to read, test and fix.

- **A move.** A retain whose value is passed to an owning parameter is removed, together with the
  binding's later release, when no path from the call reads the binding again: loop back-edges, a
  `catch`, a `finally` and a generator's resume included. An owning `Core` member leaves its subject
  with the caller on an error, so a move into it is allowed inside a `try`, and the error edge keeps
  the binding live. A Novis callee owns its parameter on every path, so a move into one is allowed
  only where no error edge reads the binding; where one does, the value is lent instead.
- **A release at the last use.** A binding never read again is released right after its last use,
  not at the end of its scope. So `$b = $a; $a = Core\Arr::slice($a, 1);` changes `$a` in place when
  `$b` is not read afterwards.
- **A lend.** `slot = M(slot, …)` lends the slot's reference to `M` with no retain, and the store puts
  the result back with no release. A slot is a property, a static property, an array element, or a
  chain of them whose containers are made sole first. Every condition holds, or the retain stays: each
  slot on the chain is plain (no hook, no `PropertyGuard`, no observer, no `lateinit` step); `M` is an
  owning `Core` member, a concatenation, or a Novis callee under the rules below; and no instruction
  between the slot's read and its store can run user code or touch the slot, read from each
  instruction's effects and not from the order of the arguments in the source. A member's callbacks
  run before anything changes, so a callback that reads the slot sees the old value; a callback whose
  effect summary writes the slot, or is "anything", keeps the retain.

**A Novis callee takes a lent value when the compiler proves it safe.** The statement keeps its
meaning: when the call fails, the slot is unchanged. The compiler sees the whole program, so it proves
that per call. It lends the slot, or a local inside a `try`, when:

- **the callee is known**: the call resolves to one body while compiling — a `private`, `static` or
  `final` method, a function, or a method no class in the program overrides. A callable and an
  interface call with more than one body keep the retain;
- **its effect summary is known**: which properties it may read or write, by class and name, solved to
  a fixed point over recursion and placed per instruction. Dynamic property access, reflection and a
  callable make it "anything", which keeps the retain.

The callee then runs a **lent body**, a second body built once per function and parameter that any
call site lends. The parameter is never freed on an error exit, because the caller still holds it. A
change to the parameter separates first, copying at that change only, when a catchable failure or a
read of the lent slot is reachable after it: both would show the old value while the new one exists,
so one copy is the minimum. Every other change is in place. A callee that may write the lent slot is
not lent: its call keeps its retain, so both values stay alive and it copies at its change. The lent
body covers a parameter that is released, changed, returned, or read by a borrowed `Core` or helper
call or an element read; any other shape gives no lent body, and the call keeps its retain. A failure
is an instruction with an error edge; a `FATAL`, such as the memory ceiling, ends the request and is
not one, and a refused append leaves its array unchanged. Every call to a Novis function is a failure,
because each may throw `RecursionError` at its entry (`rule:errors/stack-depth`), so a lent body
copies at a change after which any such call is reachable.

A local's element, `$a[k] = M($a[k])`, is not lent: a local is not a slot, so once the array is made
sole every later read of the local's old value, the error edges' releases included, would have to
read the sole array. It keeps its retain.

**Four forms the pass covers beyond a call.**

- A literal that opens with a spread of its own target, `$list = [...$list, $x]`, takes that source as
  an owning operand. A sole source that is a list is the literal itself, and the rest appends to it. A
  shared source, or one that is not a list, builds as before, because a spread renumbers number keys.
  The pass moves a local into it. A property or element keeps its copy: an item after the spread can
  fail at the memory ceiling, and its landing would free the array the slot still names.
- `Core\Arr::reduce` gives its carry away: the callback owns it as a Novis callee owns a moved
  parameter, and its error exit frees it. It is the one member that hands a value it made to a
  callback.
- A by-value `foreach` whose body writes the array only at the loop's own key, never assigns the key,
  and has no instruction that may write the array another way holds no reference of its own to it.
  The loop then walks the variable's current array, because with no reference of its own another
  holder could free the array it started on. It reads each entry before the body runs, so it sees the
  original values, and a copy of a hashed array keeps its holes so each position holds across it.
- A generator's resume takes each parked local out of the state object and leaves the field empty
  until the next `yield` parks it, so the local is its value's only reference.

**Which `Core` members own their subject: the signature decides.** A member owns its first parameter
when that parameter is `array<…>`, `string` or `bytes` and its result is the same kind, nullable or
not. There is no list and no opt-out. A member that keeps nothing of its subject still owns it, builds
its result and releases the subject. The helper's macro form declares the ABI once, the module's
symbol table returns it, and the lowering reads it there, so a wrong row can only miss a move, never
free twice. An owning helper is `check` then `apply`: `check` reads every argument borrowed, runs every
callback, reserves every allocation up to the memory ceiling and returns a plan or an error, leaving
the subject the caller's and unchanged; `apply` takes the plan and the owned subject, has no error in
its type, runs no user code, and on a shared subject builds only what it keeps.

Each owning member has two entries: its own symbol, which borrows as every helper does, and
`<symbol>_owned`, which takes the subject's reference on success only. A move into a `Core` member is
a switch of the call to the owning entry, made by the pass where the subject is released once on
every path from the call's normal exit; the error edge keeps its releases. So a call the pass does not
move costs nothing extra, and a symbol the table does not name can only be called borrowed.

**Every copy the pass keeps is reported where it is written.** The pass records each kept copy with a
reason and, where one exists, a fix, in a table that is part of the compiled program. A warm compile
cache hit still lowers the program and runs the pass, so the table is rebuilt on every run and the
cache entry carries no second copy of it. A warning is raised only where one edit removes the copy: a read of the lent slot after
the change, whose fix moves the read above it, and a catchable failure after the change, whose fix
moves the failing statement above it. Every other kept copy — two values really both used, a callable,
an overridden method, a hooked property — is a table row, shown when a recorded request ran it. The
warnings come from the pass's decisions, never from a second analysis, so `nvs check` and the
language server cannot disagree. A recorded request counts the entries or bytes each site copied.

**The guarantee is checked three ways beyond the cases.** `nvs run`, `nvs test` and `nvs serve` take a
hidden `--no-in-place` that skips the pass, so every owning call sees a shared subject and builds; `bun
nv suite --no-in-place-too` runs every case both ways and fails on any difference. In a debug build
the pass poisons a lent slot in the two windows where any read is a wrong effect decision: from the
slot's read to just before the call, where nothing in between can fail, and from the call's normal
exit to the store. A read there traps with the statement's span. The call itself is not poisoned,
because a callback the lend allows and a lent body before its first change may read the slot and
must see the old value. The in-place operations of the runtime's arrays and strings run under Miri. A
value a cache, the configuration or another request can reach always has a count above 1, so it is
never changed in place.

**What it spends.** One retain and one release per call whose subject is still read after the call,
where today it is borrowed. One more compiled body per lent function and parameter, and the effect
summaries while compiling, linear in the call graph plus the fixed point. Each owning member compiled
twice, and one retain and one release per call of its borrowed entry. A copy of a hashed array
carries its holes, at most one per live entry. One shape test per literal that spreads its target. One table per compiled program with a row per kept copy, and one flag check
per copy at run time. Nothing per request; compile time grows by one liveness pass per function.

**Declined:**
- **An in-place variant of a member, such as `Arr::sortInPlace`.** A second name for one operation is a
  thing a developer must remember to reach for, and the compiler already knows when the subject is
  sole (`rule:core-api/one-name-one-signature`).
- **Copying whenever a Novis callee is involved.** It is simple and always correct, but it makes
  `$this->items = $this->addItem($this->items, $x)` quadratic in a loop, and the compiler can prove the
  common case safe.
- **A list of owning members.** A list drifts from the helpers; the signature is already the fact.
- **Lending to a callee that writes the lent slot, with a retain in the callee and a pointer
  comparison before the caller's store.** It puts a second protocol on the store for a callee that both
  changes and replaces one value, which is rare.
- **A site id threaded through every runtime copy path to count copies.** Many runtime ABIs would
  change for the same number; the pass instead places one counting call after each kept-copy site.
