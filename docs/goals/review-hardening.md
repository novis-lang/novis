---
looper: 1
title: The rest of the 0.0.5 review is fixed or declined with its reason
summary: "Reached when a task child keeps its request's grants and memory ceiling, a decoded payload never writes a hidden slot, a restriction holds every use form the checker records, the reload command and its endpoint report what happened, a test request on the wire is armed like every other, an in-place result is the same as a built one and frees what it cut, the small time, JSON, client and editor defects are fixed, the release tags the commit it built, and every item is ticked as fixed or as declined with its reason."
context: []
stages:
  - id: 1
    title: "a task child keeps its request's grants and its memory ceiling"
    status: done
    context: [crates/nvs-runtime/src/ctx/isolate.rs, crates/nvs-runtime/src/ctx/limits.rs]
  - id: 2
    title: a decoded payload and a refused write leave no membership that points nowhere
    status: done
    context: [docs/rules/classes/serialize-is-a-closed-format.md, crates/nvs-runtime/src/revoke.rs]
  - id: 3
    title: a restriction holds every use form the checker records
    status: done
    context: [docs/rules/packaging/a-restriction-limits-where-a-declaration-is-used.md, crates/nvs-types/src/restrict.rs]
  - id: 4
    title: "the reload command says what each server did, and only the server it names answers it"
    status: done
    context: [docs/rules/config/a-reload-command-checks-now-and-waits.md, crates/nvs-cli/src/reload_cmd.rs, crates/nvs-cli/src/reload_endpoint.rs]
  - id: 5
    title: the development host check asks about the socket peer and tells the browser what to do
    status: done
    context: [docs/rules/http-server/a-development-server-answers-the-hosts-it-knows.md]
  - id: 6
    title: "a test request on the wire is armed like every other, and --since never runs fewer tests than the change reaches"
    status: done
    context: [crates/nvs-cli/src/runner/reach.rs, crates/nvs-cli/src/runner/since.rs]
  - id: 7
    title: "an in-place result equals the built one, and a cut frees what it no longer needs"
    status: done
    context: [docs/rules/types/a-replaced-value-is-changed-in-place.md]
  - id: 8
    title: the time classes read their text and their epoch one way
    status: done
    context: [docs/rules/core-classes/a-time-class-builds-its-own-values.md]
  - id: 9
    title: a typed decode bounds its issue list, and a passkey challenge is taken once
    status: done
    context: []
  - id: 10
    title: "a chunked reply is pooled after its last byte, and a worker panic stays on the worker"
    status: done
    context: [crates/nvs-lsp/src/workers.rs]
  - id: 11
    title: a release refuses a tip that holds shipped code its build did not have
    status: open
    context: [tools/nv/cmd/release.ts]
  - id: 12
    title: "the features that moved have their proofs, their pages and their change entries"
    status: open
    context: [website/src/data/changes.json]
  - id: 13
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: task-child-limits
    stage: 1
    name: "a task of a narrowed script is refused the capability the script lacks, and a task's allocation over the ceiling is refused before it is made"
    run: [cargo, test, --test, task_child_limits]
    timeout: 1800
  - id: hidden-slot
    stage: 2
    name: "a payload that names a value for a revocable object's hidden slot is refused, and a write the ceiling refused links nothing"
    run: [cargo, test, --test, hidden_slot]
    timeout: 1800
  - id: restrict-use-forms
    stage: 3
    name: "every expression kind that can reach a declaration has a row, and a restricted one outside its places does not compile"
    run: [cargo, test, --test, restrict_use_forms]
    timeout: 1800
  - id: reload-outcomes
    stage: 4
    name: "a server that closes with no reply, panics in its check or is not the process its file names makes nvs reload exit nonzero"
    run: [cargo, test, --test, reload_outcomes]
    timeout: 1800
  - id: rebinding-guard
    stage: 5
    name: "an unknown host from a loopback socket is a 421 whatever a forwarded header says, and its body names development_hosts"
    run: [cargo, test, --test, rebinding_guard]
    timeout: 1800
  - id: wire-request-armed
    stage: 6
    name: "a request to a test's own server keeps its mail, reads the test's clock and environment, and its logs are recorded"
    run: [cargo, test, --test, wire_request_is_armed]
    timeout: 1800
  - id: since-selection
    stage: 6
    name: "a deleted file, a renamed file and a file outside the folder each select the tests that reached them"
    run: [cargo, test, --test, since_selection]
    timeout: 1800
  - id: in-place-results
    stage: 7
    name: "a cut after mapKeys or flip appends at the key the built array appends at, a short slice frees its long subject, and a queue loop grows linearly"
    run: [cargo, test, --test, in_place_results]
    timeout: 1800
  - id: time-text
    stage: 8
    name: "a zone offset with two signs is refused, and an instant before 1970 gives the second and millisecond fromEpoch built it from"
    run: [cargo, test, --test, time_text]
    timeout: 1800
  - id: decode-and-passkey
    stage: 9
    name: "a list of many bad elements ends in a bounded issue list, and two parallel finishes of one challenge give one success"
    run: [cargo, test, --test, decode_and_passkey, --lib, --, bounded_issue_list, exactly_the_bound_lists_every_issue, two_requests_that_take_one_key_get_its_bytes_once_on_any_store]
    timeout: 1800
  - id: chunked-end
    stage: 10
    name: "a reply whose last chunk and trailer arrive in two segments leaves nothing for the next call on its connection"
    run: [cargo, test, --lib, --, a_reply_whose_last_chunk_and_trailer_arrive_in_two_segments, a_pooled_connection_with_bytes_waiting_is_closed_when_it_is_taken]
    timeout: 1800
  - id: worker-panic
    stage: 10
    name: "a part that panics on a worker is logged, its entry is skipped, and the server answers the next request"
    run: [cargo, test, --test, worker_panic]
    timeout: 1800
  - id: release-fixes-built
    stage: 11
    name: "a tip with a commit that changes shipped code the build did not have fails the release, whatever the commit's type"
    run: [bun, test, tools/nv/test/release-fixes-built.test.ts]
    timeout: 300
  - id: proofs
    stage: 12
    name: "every feature the goal's work reached has its proofs"
    run: [bun, nv, proofs, --gate, --goal, review-hardening]
    timeout: 1800
  - id: close
    stage: 13
    name: "the goal-end check is green: ci --check's checks of the tree's files, the owners and the goal's proofs"
    run: [bun, nv, close, --goal, review-hardening]
    timeout: 900
---

# The rest of the 0.0.5 review is fixed or declined with its reason

## Why

A review of the work between `v0.0.4` and `v0.0.5` found more than the four holes goal `safety-holes`
closes. This goal takes the rest. Each item says how sure the review is:

- **run**: a program or a measurement showed it. The figure is in the item.
- **read**: the code at the anchor was read and says what the item says. Nothing was run.
- **check first**: one reviewer reported it and nobody else read it. The item starts by reproducing
  it.

Two of them are older than `v0.0.4` and are security holes all the same. `Ctx::child`
(`crates/nvs-runtime/src/ctx/isolate.rs:396`) never copies `grant_filter`, and a context with no
filter is allowed everything its configuration grants, so a script started with `grants:` gets every
capability back inside `Core\Task::all`. The same child never arms the allocator's ceiling. Outside a
task, `Core\Str::repeat('x', 400000000)` is refused before it is made, with 2,780 bytes held. Inside
`Core\Task::all` it is made, and the limit is noticed afterwards with 401,054,350 bytes held against a
ceiling of 268,435,456.

Three measured costs came with the in-place work. `mapKeys`, then `withoutLast`, then `$a[] = 30`
gives the keys `0,2` in place and `0,1` under `--no-in-place`. 400 slices of 8 bytes taken from
strings of 1 MB end at the memory limit with 267,024,012 bytes held, and pass under `--no-in-place`. A
queue loop of `first`, `withoutFirst` and `append` takes 993 ms for 5,000 rounds, 4,742 ms for 10,000
and 15,837 ms for 20,000.

## Standing decisions

**An item that says *check first* gets one attempt at a reproduction.** A test or a probe under
`.agent-tmp/` that shows it comes before any fix. If one attempt does not show it, the item is ticked
as "not found", with one line under `### State` that says what was tried. Nothing is built for it.

**A task child is part of its request for every limit.** `Ctx::child` copies `grant_filter`, and the
scheduler arms the request's memory ceiling on a task's stack as it does on the request's. Reason: a
narrowing or a ceiling that a task escapes is not one. It costs one `Arc` clone per spawn and nothing
per allocation. Safe default if arming per switch is not possible: a task's allocation asks the
request's reading through its `Ctx` before it is made.

**The hidden slot of a revocable object is never read from a payload and never written to one.**
`Core\Serialize::encode` skips it, and `decode` gives it its fresh value and reads no node for it. A
payload written before this change holds a node for that slot, and decoding it throws the closed
format's error, as any payload of another layout does
(`rule:classes/serialize-is-a-closed-format`). Declined: reading and checking the node, because no
value from outside is a valid membership pointer.

**A collection links a member only after the write landed, and unlinks by identity.** A write the
ceiling refused links nothing. `remove_entry` removes an entry only when the value under the key is
the revoked object.

**`resolved` matches every `ExprInfo` variant by name, with no `_` arm.** Each variant either lists
the declarations it uses or says in a comment why it uses none. Restriction, deprecation and the test
code check read this one function, so a variant added later does not compile until it is judged. A
union receiver is a use of the member on every arm. `Core\Program::implementing`, `implementingWith`
and `constructors` are a use of each class they construct. `static::NAME` is a use of the constant. A
property default, a parameter default and an attribute payload are checked like a body.

**A restriction on a class is not a restriction on calls through its ancestors.** A call through an
interface or a parent type is a use of that type's member, and the rule's fragment says so in one
sentence. Reason: the checker cannot know which body runs. `new $cls()` through `class<Base>` is held
by the entries on `Base` for the same reason. Nothing is built for either.

**A package's `[[restrict]]` may name only its own declarations and `Core`'s.** A `use` target under
neither the package's own name nor `Core` is refused when `package.toml` is read. The application's
file may name anything. Reason: one package must not add compile errors to another package's code
or the application's. `Core` stays allowed because a framework that wraps a `Core` method restricts
it, which is the feature's documented example (`Core\Router::urlSigned`).

**`nvs reload` exits 0 only when every server it found replied and applied everything.** A server
that drains writes a line that says it is stopping. A connection that closes with no line is a failed
server: the command names it and exits 1. A panic in a server's check is caught and replied as a
failure. The rule's exit table gains "or did not answer" on the line for 1. The accept loop logs an
accept error and goes on. One deadline in `reload_endpoint::answer` bounds a connection on both
platforms. The command reads at most 64 KiB of a reply and prints control characters escaped.

**On Windows the command checks which process serves the pipe.** `GetNamedPipeServerProcessId` must
give the process id in the endpoint file's name. Otherwise the server is reported as unreachable, and
its file is left alone.

**A cache folder and its files are made private.** A shard folder is created with the helper that
makes `run/` (`crates/nvs-config/src/data.rs:765`), and an artifact with mode `0600` on Unix. A
program's file or socket call is refused under the data folder's `cache/` and `run/`, whatever a
grant's prefix covers.

**The development host check asks about the socket peer.** A request whose socket peer is loopback is
asked, whatever `X-Forwarded-For` gives as the client. The 421 answer has a text body in development
that names `[server] development_hosts` and the host to add. The log line is written once per host
per minute, with the host in its escaped form.

**One function arms a test context.** The clock, the environment, the records and the routes are set
by one function with four callers (`crates/nvs-cli/src/runner.rs:1697`, `:1793`, `:3963`, `:4583`), so
a request to a `#[Test(server: true)]` listener keeps its mail and reads the test's clock.

**`--since` runs more tests when it cannot tell, never fewer.** A deleted file selects every test the
stored map says reached it. A rename is read as a delete and an add (`git diff --no-renames`). A
changed path outside the program folder, or one that cannot be resolved, runs every test. A run that
skipped tests keeps the stored hash of every function any of those tests reached, of each of their
classes, and of every file, and keeps a stored hash for what is gone. A function reached by both a
run and a skipped test keeps the old hash too, so the run test runs once more. It costs at most one
extra run of a test, and the map converges at the next `--since` run. A directory of programs has
the folder it is in as its program folder (`app::folder` of the directory), so `ours` is wider for
it, which runs more tests, never fewer.

**The members of `Core\Test` that answer for the outside world throw in work a server runs.**
`answerHttp`, `answerSocket`, `tlsSession` and `scriptAnswers` throw a `LogicError` in a request, a
connection, a scheduled fire and a job outside `nvs test` (`not_in_a_server` in
`crates/nvs-stdlib/src/test.rs`). A test, a request or run a test makes, and a CLI program may call
them. Reason: refusing them everywhere outside `nvs test` would break hundreds of examples, attacks
and benches that answer their own calls under `nvs run`, and a CLI program that fakes its own calls
harms only itself. The rule's Declined list says so. A public, non-static default method of an
interface is always marked overridden (`crates/nvs-types/src/signatures.rs`), so a double's body for
it runs. It costs one method lookup per call to an interface's default method.

**The built array is the reference for the in-place one.** After any member, the next appended key is
the one the build path gives, whatever the storage. `crates/nvs-fuzz-checks/src/in_place.rs` gains a
member, then a cut, then an append as a shape.

**A cut keeps its allocation only when it keeps at least half of it.** A sole string cut to less than
half of its capacity is copied into an allocation of its own size, and the old one is freed. It costs
a copy of at most half the old string, and it bounds what a kept slice holds to twice its length.

**A map's append counter is kept, not recounted.** `withoutFirst` and `append` on a map keep the
counter the array already has, so the queue loop costs the same per round at every size. The stage
measures the three sizes again and writes them under `### State`.

**An epoch reader floors.** `toEpochSeconds`, `toEpochMillis` and `toEpochMicros` give the largest
whole count at or before the instant, so `fromEpoch` of the result and the remainder builds the same
instant. Before 1970 that is one less than today's result. A zone offset takes one sign, and its hour
and minute are digits only. `Date::fromIso('-0000-01-01')` throws. `Zone::of` and a written zone keep
the name as the database spells it. `Date::at` names its parameters `year`, `month` and `day`.

**A removed member names its replacement.** `Core\Session::start`, `set`, `setSecret`, `clear` and
`destroy`, and `Core\Time\DateTime::Weekday`, get rows in `MOVES`
(`crates/nvs-stdlib/src/moved.rs:51`). `set` gets a help line with both `setShortLived` and
`setLongLived`. `E0763` on a `Core` attribute says only "delete one".

**A typed decode stops collecting issues at 64.** The 65th issue ends the list with one line that says
more follow, and the loop asks the memory budget before each issue it keeps.

**A passkey challenge is removed in the store when it is taken.** `take_challenge` deletes the key in
the store at once and succeeds only when the store says it removed it. It costs one store round trip
on a passkey finish. A session's stored clock is clamped to now plus its lifetime when it is saved.

**The outbound client pools a chunked connection after the final line of its trailer.** A connection
with bytes left on its socket when it is taken from the pool is closed.

**A panic on an editor worker stays there.** The server logs it with the file's path, skips that
entry until the file changes, and never analyses the entry again on its own thread. `adopt` and
`take_made` run inside the guard.

**A release refuses a tip that holds shipped code its build did not have.** `--fixes-built` reads
every commit between the built commit and the tip, whatever its type, and fails when one changes a
path under `crates/`, `extensions/`, `vendor/`, `Cargo.toml` or `Cargo.lock`. The attestation check of
`release-promote.yml` names `--signer-workflow`. This goal runs no release and changes no secret.

**What this goal does not take.** The sessions a CSRF token makes for visitors with no cookie get a
gap record under `data/gaps/`, with the count stage 9 measures, because the fix is a design of its
own. The same zone text is accepted at a `Zone` parameter and refused by `Zone::of`: that is the
written rule, and it stays. No cost the review only read is here: the compile cost of the ownership
pass, the session script's bytes, the SQLite save's hops and the CI cache's evictions are each
measured before a goal takes them.

**What stops it.** One attempt per item, and two sessions per stage, counted in `### State`. At two
sessions a stage's open items move to `### Backlog` as declined, each with its reason, and a security
item among them gets a gap record. No stage gains an item.

**Lanes.** Stages 1 and 2 share `crates/nvs-runtime` and run in order. Stages 3, 4, 5, 6, 8, 9, 10 and
11 touch disjoint files and may run as side goals. Stage 7 follows goal `safety-holes`, which changes
the same in-place code. Gate: `crates/nvs-codegen/tests/operand_moves.rs` exists.

## Stages

### 1. a task child keeps its request's grants and its memory ceiling

- [x] `crates/nvs-runtime/src/ctx/isolate.rs:396`: `Ctx::child` copies `grant_filter`. *read.*
- [x] `crates/nvs-runtime/src/ctx/limits.rs:368`: a task's stack is armed with its request's ceiling
  (`arm_on_this_stack`, called at `crates/nvs-host/src/scheduler.rs:1013`). The comment at
  `crates/nvs-runtime/src/ctx/isolate.rs:454` says what holds. *run.*
- [x] `crates/nvs-cli/tests/task_child_limits.rs:1`: a script spawned with `grants: ['fs.read']`
  under a configuration that grants `net` too is refused an outbound call inside `Core\Task::all`. An
  allocation of 400 MB inside a task is refused before it is made. It runs once, by name.
- [x] `tests/hostile/`: one attack for each, beside the existing attacks on `grants:` and on the
  memory limit. `bun nv proofs --id 'Core\Task::all'` names the folder.
- [x] `cargo build` and `cargo clippy --all-targets -- -D warnings` are green.

### 2. a decoded payload and a refused write leave no membership that points nowhere

- [x] `crates/nvs-runtime/src/graph.rs:1159`: `decode` reads no node for the hidden slot, and `encode`
  at `:870` writes none. `rule:classes/serialize-is-a-closed-format` *read.*
- [x] `crates/nvs-stdlib/src/objmap.rs:540`: the link follows a write that landed. *check first.*
- [x] `crates/nvs-stdlib/src/objmap.rs:613`: `remove_entry` compares the value's identity.
- [x] `crates/nvs-runtime/src/graph.rs:595`: a live copy that adopts a collection unlinks the source's
  members. *check first*: find the syntax that reaches `copy_graph_to` at `:718` with one holder.
  Not found: see `### State`.
- [x] `crates/nvs-runtime/src/record.rs:408`: a dump of a revocable object starts after the hidden
  slot.
- [x] `crates/nvs-codegen/tests/hidden_slot.rs:1`: a payload built by the test with a value node at
  the hidden slot is refused, a round trip of a revocable object in a map works, and a `set` at the
  ceiling followed by a revoke touches no freed collection. It runs once, by name.
- [x] `crates/nvs-stdlib/src/objset.rs:584` and `crates/nvs-stdlib/src/heap.rs:867`: `add` and `push`
  link only a write that landed, with a row each in `hidden_slot.rs`.
- [x] `cargo build` and clippy are green.

### 3. a restriction holds every use form the checker records

**Split:** an `Explore` subagent lists every `ExprInfo` variant of
`crates/nvs-types/src/expr_table.rs`, one line each, with whether `resolved` has an arm for it.

- [x] `crates/nvs-types/src/deprecated/template.rs:654`: `resolved` has no `_` arm, as the standing
  decision gives it. `rule:packaging/a-restriction-limits-where-a-declaration-is-used` *read.*
- [x] `crates/nvs-types/src/program.rs:106`: the three `Core\Program` members record the classes they
  construct. *check first.* Shown: an enumeration of a restricted class compiled. Fixed in `resolved`.
- [x] `crates/nvs-types/src/defaults.rs:430`: a property default, a parameter default at `:556` and an
  attribute payload (`crates/nvs-types/src/attributes.rs:979`) are read inside a window. *check first.*
  Shown: each of the three compiled with a restricted constant. Fixed by `record_default_use` and a
  window around each attribute.
- [x] `crates/nvs-types/src/attributes.rs:147`: an attribute on an anonymous function's parameter is
  checked like a method parameter's. `fn(#[NoSuchThing(x: 1)] int $n): int => $n` compiles today.
  *run.*
- [x] `crates/nvs-config/src/restrict.rs:371`: a package's `use` target is its own or `Core`'s.
  - [x] the check is in `parse`
  - [x] a row in `crates/nvs-config/tests/restrict.rs`: a package naming `Shop\Cart` is E0665, one
    naming `Core\Str::upper` is not
- [x] `crates/nvs-types/src/restrict.rs:718`: an allowed place that is a class does not allow a
  namespace of the same name. *check first.* Shown by a row, and fixed with `Site::graph`.
- [x] `crates/nvs-types/src/retrieval.rs:452`: a retrieval binds `static` to the method's owner.
  *check first.* Not found: through a subclass the fields are `property<Sub>`, and every key valid
  for the declaring class is valid for the subclass.
- [x] `crates/nvs-cli/tests/restrict_use_forms.rs:1`: one row per variant the subagent listed, in a
  two-package fixture named `Shop` and `Framework`. It runs once, by name. It is in `nvs-cli`
  because a row runs `nvs check` over a folder with a `package.toml`.
  - [x] a union receiver, an enumeration, `static::NAME`, a class that is not a namespace
  - [x] a route link, and the rows the defaults, attribute and retrieval items add
- [x] `docs/rules/packaging/a-restriction-limits-where-a-declaration-is-used.md:57`: the use forms as
  the table has them, and the sentence on ancestors.
- [x] `cargo build` and clippy are green.

### 4. the reload command says what each server did, and only the server it names answers it

- [x] `crates/nvs-cli/src/reload_cmd.rs:103`: no reply is a failure, and `exit_code` at `:208` counts
  it. `rule:config/a-reload-command-checks-now-and-waits` *read.*
- [x] `crates/nvs-cli/src/reload_endpoint.rs:292`: a check panic is a reply, a drain writes its line,
  the accept loop at `:133` goes on after an error, and `answer` at `:169` holds the one deadline.
- [x] `crates/nvs-cli/src/reload_cmd.rs:102`: the bounded read, and escaped text at `:270`.
- [x] `crates/nvs-host/src/endpoint/windows.rs:127`: the server process check. *check first.*
- [x] `crates/nvs-cli/src/cache.rs:1447`: a private shard folder, and a private artifact at `:1584`.
  *check first*, in WSL with `bun nv linux-test`.
- [x] `crates/nvs-runtime/src/capability.rs:102`: `require_as` refuses a path under the data folder's
  `cache/` and `run/`. *check first.*
- [x] `crates/nvs-cli/tests/reload_outcomes.rs:1`: the check's three cases and a reply of 1 MiB. It
  runs once, by name, and in WSL with `bun nv linux-test`.
- [x] `docs/rules/config/a-reload-command-checks-now-and-waits.md:45`: the exit table.
- [x] `cargo build` and clippy are green.

### 5. the development host check asks about the socket peer and tells the browser what to do

- [x] `crates/nvs-cli/src/serve.rs:1194`: `table.select` gets the socket peer for the host check, and
  `refuses_host` (`crates/nvs-server/src/mount.rs:438`) reads it.
  `rule:http-server/a-development-server-answers-the-hosts-it-knows` *read.*
- [x] `crates/nvs-server/src/serve.rs:849`: the 421 body in development.
- [x] `crates/nvs-server/src/trace.rs:468`: the line once per host per minute, with the host escaped.
- [x] `crates/nvs-cli/tests/rebinding_guard.rs:1`: loopback in `trusted_proxies`, an unknown host and
  `X-Forwarded-For: 203.0.113.9` is a 421 with the body. It runs once, by name.
- [x] The rule's "Who is asked" paragraph, and `crates/nvs-config/src/default.toml:608`.
- [x] `cargo build` and clippy are green.

### 6. a test request on the wire is armed like every other, and --since never runs fewer tests than the change reaches

- [x] `crates/nvs-cli/src/runner.rs:4583`: the one arming function and its four callers. *read.*
- [x] `crates/nvs-stdlib/src/test.rs:2654`: the four members throw outside a test. *check first.*
- [x] `crates/nvs-ir/src/lower/expr.rs:5233`: `late_bound` holds for a program that makes classes at
  run time. *check first*, with a double that gives an interface's default method.
- [x] `crates/nvs-stdlib/src/test.rs:3442`: a `sendFile` body is read within the request's memory
  limit (`crates/nvs-runtime/src/host.rs:744`).
- [x] `crates/nvs-cli/src/runner/reach.rs:499`: a deleted file and a missing function select their
  tests, and `record` at `:458` keeps the hashes of what unrun tests reached.
- [x] `crates/nvs-cli/src/runner/since.rs:134`: `--no-renames`, and a path outside the folder or one
  that does not resolve at `:151` runs every test.
- [x] `crates/nvs-cli/tests/wire_request_is_armed.rs:1` and
  `crates/nvs-cli/tests/since_selection.rs:1`. Each runs once, by name.
- [x] `cargo build` and clippy are green.

### 7. an in-place result equals the built one, and a cut frees what it no longer needs

- [x] `crates/nvs-stdlib/src/arr.rs:3419`: `kept_counter` and the in-place `rekey`
  (`crates/nvs-runtime/src/array.rs:1265`) give the build path's next key.
  `rule:types/a-replaced-value-is-changed-in-place` *run.*
- [x] `crates/nvs-runtime/src/string.rs:1456`: `keep` copies out under half. *run.*
- [x] `crates/nvs-runtime/src/array.rs:1037`: the counter is kept, with no walk. *run.* The loop of
  § *Why* at 5,000, 10,000 and 20,000 rounds, in a scratch file under `.agent-tmp/`, on
  `target/release/nvs.exe` after `bun nv relink --build`.
- [x] `crates/nvs-runtime/src/array.rs:2639`: the budget is asked about the store that is allocated.
  *check first.*
- [x] `crates/nvs-fuzz-checks/src/in_place.rs:1`: the new shape.
- [x] `crates/nvs-codegen/tests/in_place_results.rs:1`: the three programs of § *Why*. The queue case
  counts allocations or key parses per round at two sizes, never time. It runs once, by name.
- [x] `cargo build` and clippy are green.

### 8. the time classes read their text and their epoch one way

- [x] `crates/nvs-stdlib/src/time.rs:4168`: `parse_offset` takes one sign and digits. `'+-5:00'` gives
  a `-05:00` zone today. `rule:core-classes/a-time-class-builds-its-own-values` *run.*
- [x] `crates/nvs-stdlib/src/time.rs:4592`: the three epoch readers floor.
  `Instant::fromEpoch(-1, {nanos: 500000000})->toEpochSeconds()` gives `0` today. *run.*
- [x] `crates/nvs-stdlib/src/time.rs:3775`: `-0000` throws, and a zone keeps the database's name at
  `:4261` and `:4718`.
- [x] `crates/nvs-stdlib/src/time.rs:3725`: `Instant::fromIso` takes only the forms the rule lists.
  *check first*, with `23:59:60Z`, a lower-case `t` and a comma.
- [x] `crates/nvs-stdlib/src/time.rs:2691`: `Date::at(year, month, day)`.
- [x] `crates/nvs-stdlib/src/moved.rs:51`: the `MOVES` rows, and `E0763`'s help at
  `crates/nvs-diagnostics/src/code/types_e07.rs:1144`.
- [x] `crates/nvs-codegen/tests/time_text.rs:1`. It runs once, by name.
- [x] `cargo build` and clippy are green.

### 9. a typed decode bounds its issue list, and a passkey challenge is taken once

- [x] `crates/nvs-stdlib/src/json.rs:5577`: the bound of 64 and the budget ask. *check first*, with a
  list of 1,000,000 bad elements under the default limit.
- [x] `crates/nvs-stdlib/src/json.rs:820`: the `isValid` card and
  `docs/examples/core/Json/isValid/about.md:1` say what `CHANGELOG.md` says.
- [x] `crates/nvs-stdlib/src/passkey.rs:804`: `take_challenge` removes in the store
  (`take_bytes`, `crates/nvs-stdlib/src/session.rs:2101`). *check first*, with two parallel finishes.
- [x] `crates/nvs-stdlib/src/session.rs:803`: the clamp, in the script and in
  `crates/nvs-stdlib/src/session/sqlite.rs:452`.
- [x] `data/gaps/`: the record for sessions a CSRF token makes (`sent_id`,
  `crates/nvs-stdlib/src/session.rs:2047`), with the rows 1,000 requests with no cookie make.
- [x] `crates/nvs-config/src/tree.rs:1544`: the comment says what the code does.
- [x] `crates/nvs-codegen/tests/decode_and_passkey.rs:1`. It runs once, by name.
  - [x] The decode half: `crates/nvs-codegen/tests/decode_and_passkey.rs`, two cases, passed.
  - [x] The passkey half: the acceptance command also runs the unit test
    `two_requests_that_take_one_key_get_its_bytes_once_on_any_store`, whose store helpers are
    private to `nvs-stdlib`.
- [x] `cargo build` and clippy are green.

### 10. a chunked reply is pooled after its last byte, and a worker panic stays on the worker

- [x] `crates/nvs-stdlib/src/http/transport.rs:1014`: `dechunk` ends after the trailer's last line,
  and `pool::take` (`crates/nvs-stdlib/src/http/pool.rs:76`) closes a connection with bytes waiting.
  *check first.*
- [x] `crates/nvs-lsp/src/index.rs:1234`: no `analyse_here` after a part that panicked, and the path
  at `:1035`. *read.*
- [x] `crates/nvs-lsp/src/workers.rs:572`: `adopt` and `take_made` inside the guard, and the log line.
- [x] `crates/nvs-stdlib/tests/chunked_end.rs:1` and `crates/nvs-lsp/tests/worker_panic.rs:1`, the
  second with a test-only switch that makes one file's check panic. Each runs once, by name.
  - [x] The chunked half: two unit tests in `transport.rs`'s tests, passed; `chunked-end` runs them
    by name, because `Framed` and the pool are private to `nvs-stdlib`.
  - [x] The worker half: `crates/nvs-lsp/tests/worker_panic.rs`, one case, passed. The switch is
    `nvs_lsp::panic_on_worker`.
- [x] `cargo build` and clippy are green.

### 11. a release refuses a tip that holds shipped code its build did not have

- [ ] `tools/nv/cmd/release.ts:26`: `--fixes-built` as the standing decision gives it, at `:1099`.
  *check first*: read `.github/workflows/release.yml:269` and say under `### State` what the tag is
  made on.
- [ ] `.github/workflows/release-promote.yml:207`: `--signer-workflow` on the attestation check.
- [ ] `tools/nv/test/release-fixes-built.test.ts:1`. It runs once, by name.
- [ ] `bun nv ci --check` is green.

### 12. the features that moved have their proofs, their pages and their change entries

- [ ] `website/src/data/changes.json:1`: `changed` for `Core\Time\Instant::toEpochSeconds`,
  `toEpochMillis` and `toEpochMicros`, `Core\Time\Date::at`, `nvs reload`, `[server]
  development_hosts`, `Core\Serialize::decode`, `Core\Test::answerHttp` and its three siblings, and
  `[[restrict]]`.
- [ ] The `about.md` and help of each: reread against what was built and stamped with
  `bun nv site --stamp`. The reload page says what exit 1 means.
- [ ] `bun nv proofs --owed --goal review-hardening`, and every proof it names is paid.
- [ ] `bun nv site --check stale`, `changes` and `review` are green.

### 13. the goal is verified once, and every failure is fixed

- [ ] The newcomer review of [prose.md](../agent/prose.md) § *A newcomer reads it* over the texts
  stage 12 wrote.
- [ ] Run the check the pack names for this stage: `looper verify` at a full gate, or a build and the tests this goal wrote, by name, at a close.
- [ ] Only if it is red: fix every failure it names in one pass, never deleting or loosening a test, then run it once more, and it is green.

## Handoff

### State

Stages 1 to 8 are done, each with its acceptance test passing. `Live::adopt` and the retrieval
`static` item were not found. Stage 7 is summarised in its commits (`in_place_results`); the queue
loop measured 66, 60 and 66 ms at 5,000, 10,000 and 20,000 rounds.

Stage 8 is summarised in its commits (`time_text`). Stage 9 landed its code in one session (three commits):

- *Check first* (decode) was shown by a probe: 1,000,000 bad elements filled the 256 MB limit with
  issues. `gather` and `MAX_ISSUES` in `json.rs` now bound every frame. The 65th issue ends the list
  with one issue at path `""`, and the message says "more than 64". The rule fragment
  `derive-reports-every-field` records the bound.
- *Check first* (passkey) was read from the code, not reproduced: `take_bytes` removed the key only
  in the request's copy. A new store operation `take` (`TAKE` script, SQLite `take`) deletes the key
  only when the store still has the bytes this request loaded. The SAVE script gained `ARGV[4]`, the
  long ceiling, and each later index moved by one. Tests: `two_requests_that_take_one_key_get_its_
  bytes_once_on_any_store` and `a_save_brings_a_clock_stored_far_ahead_down_to_its_lifetime_on_any_
  store` in `session.rs`, passed; the 16 `on_any_store` cases passed.
- The CSRF gap record is `data/gaps/nvs-stdlib/csrf-token-makes-a-session-per-visitor.json`, owned
  by this goal. Its count, 1,000 heads, is read from the code. A probe under `nvs serve` returned 500
  with no log reason, and was not followed.

Sessions per stage: stage 1, one; stage 2, two; stage 3, two; stage 4, two; stage 5, one; stage 6,
two; stage 7, one; stage 8, one; stage 9, one.

### Next

Stage 9's last item: the acceptance `decode-and-passkey` runs `cargo test --test decode_and_passkey`
(`crates/nvs-codegen/tests/decode_and_passkey.rs`), which has only the decode half. Its fixtures have
no session store. Either add a passkey case there that drives two contexts over one stored session,
or change the check's command to also run the unit test
`two_requests_that_take_one_key_get_its_bytes_once_on_any_store` (pick the second if the first needs
`pub` items in `nvs-stdlib`). Then set stage 9 done. Stages 10 and 11 touch disjoint files.

Stage 13's `owners` must give the CSRF gap record another owner before this goal's file is deleted.
The rule id `a-session-store-answers-five-operations` and `docs/ground-rules.md:991` still say five
operations; its text now lists six. Rename it in stage 12 or 13 with every citation.

Stage 12 owes `tools:cli/nvs-test` its change entry and page reread for the `--since` change, and
`Core\Arr::append` its change entry, its reference card reread and `website/src/data/core.json`
regenerated for the counter change and the new help text. From stage 8 it owes change entries and
card rereads for `Core\Time\Instant::toEpochSeconds`, `toEpochMillis`, `toEpochMicros`, `fromIso`,
`Core\Time\Date::at` (parameter names; `core.json:49544` still says `$y, $m, $d`),
`Core\Time\Date::fromIso` and `Core\Time\Zone::of`, and the example
`docs/examples/core/Time-Date/at/01-building-a-date.nvs` reread.

Tests that may go red: any `session.rs` or `sqlite.rs` test that reads the SAVE script's arguments
by index, or a stored clock past now plus its lifetime; a passkey or session case whose second
request takes a key another request already took; a JSON case with more than 64 issues, whose
message now says "more than 64"; the `Core\Json::decodeAs` and `isValid` cards' change. Also
`crates/nvs-stdlib/src/moved.rs`'s
`every_move_points_from_a_missing_name_to_a_registered_one` was not run; it checks the six new rows
against the registry. A test or example that calls `Instant::fromIso` with a leap second, a
lower-case `t` or `z`, a comma before the fraction, or a basic offset such as `+0100`, now throws. A
program that prints a zone built from a name written in another case now prints the database's
case. A test of `toEpoch*` before 1970 with a part of a unit now gets one less. A test that names
`Date::at`'s arguments `y`, `m`, `d` no longer compiles. An `E0763` case that reads the old help on
a `Core` attribute. A program that calls `Core\Arr::append` on an array that lost its highest
integer key (by `unset`, `withoutLast`, `filter` or a slice) now appends after the removed key, as
`$a[] = $v` does. A test that reads a sole string's address or capacity after a short cut. A
`mapKeys` or `flip` result whose new keys are `0` to `n-1` is now a list, so a bench or a count of
its storage may move. A `--since` case in `crates/nvs-cli/tests/test_command.rs` whose checkout
holds a changed file outside the program's folder now runs every test. A filtered run beside
`--since` now leaves the hashes of the tests it skipped as they were, so the next `--since` run may
select more tests than before. A program served by `nvs serve`, or a job or schedule, that calls
`Core\Test::answerHttp`, `answerSocket`, `tlsSession` or `scriptAnswers` now throws. A
`Core\Test::request` of a route that sends a file larger than the test's memory limit now ends on
that limit. A count of virtual calls, or a bench, over an interface's default method may move. A
`reload_command.rs` case that expected exit 2 with only an unreachable
server now gets 1. Its `state` for a closed connection is `silent`, no longer `stopped`. A
deprecation or `E0889` case whose program reads a deprecated or test-code
constant in a property default, a parameter default or an attribute payload now warns or fails
there. An attribute on an anonymous function's parameter that names no attribute declaration, or
one that does not fit a parameter, now fails to compile. An editor completion or `E0664` case that
relied on a class's allowed name matching a namespace of the same name. Stage 1's arming may move
where a task's allocation over the ceiling stops in a conformance or group test. A dump of a
revocable object no longer prints a `#revocable` line. A test or example whose program is granted
a folder above the data folder and reads or writes under its `cache/` or `run/` now throws. A
test that expects a bodiless `421`, or a `421` from a loopback proxy to pass through, now fails.

### Backlog

- A release of the last reference to a suspended generator runs its `finally` bodies
  (`crates/nvs-runtime/src/object.rs:5126`, `unwind_abandoned`). An `InstKind::Release` counts as no
  program code (`rule:types/a-replaced-value-is-changed-in-place` § *What a quiet call is*), so a
  lend may cross a release that runs a `finally` which writes the lent slot. Found by
  `safety-holes`' stage 2 reader; not reproduced.
- A class constant's initializer, `public const int N = Framework\Base::LIMIT;`, is folded in the
  signatures pass and records no `ExprInfo`, so a restricted constant there is probably not checked.
  It is the same shape as stage 3's defaults item and was not part of it. Not reproduced.
