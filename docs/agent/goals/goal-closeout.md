---
milestone: post-parity
position: last
---
# Loop goal 183 — every old goal is closed, its decisions are kept, and the goals are deleted

Goal `ci-green` ends the program the chain was written for. This goal does three things, in order,
before the performance pass starts. First, it proves that every goal in front of it is really
finished. Second, it makes **a finished goal is deleted** the rule for every goal from here on.
Third, it deletes the old goals and everything that cited them.

When it is reached, the chain holds this goal and goal `performance-pass` and nothing else.
`docs/agent/goal-decisions.md` keeps every decision that only a goal's prose held, so nothing is lost
with the files. That file is for reading, not a rule. The test suites and `nv verify` are what protect
finished work from now on, and no carried floor does.

## Why here

**Behind goal `ci-green`, by the user's decision of 2026-10-01.** The audit asks whether the whole
program is finished. That question has an answer only once the last goal of the program is green, and
CI is that last goal.

**The audit and the deletion are one goal, not two.** Goal `ci-green`'s check is carried in this goal's
floor. It asks whether the code `HEAD` holds is what CI last ran. The first commit here that is not
bookkeeping turns it red, and only a push turns it green again. Two goals would mean a `BLOCKED` for a
push between the audit and the deletion. In one goal the check leaves with goal `ci-green`'s record
in Stage 6, and nothing waits on the user.

**The rule change is here, in the first new goal, by the user's decision of 2026-10-01.** Deletion is
the rule from this goal on, and never for a goal before it. Stage 5 lands it before Stage 6 uses it, so
the old goals are deleted by the same code that deletes every later goal. The driver then deletes this
goal itself when it switches to goal `performance-pass`.

**What this spends:** the carried floor. Today every walked goal's checks are run again for every goal
behind it. After this goal nothing is carried. The user's call is that the suites, the `.nvst` trees and
`nv verify` already cover everything a goal's checks proved. A check that proved something no suite
runs is found in Stage 3 and becomes a test before its goal is deleted.

## Stage 0 — the catch-up

None.

## Stage 1 — the floor

Goal `ci-green`'s whole carried list, run by the driver as it is.

**Its own check goes red at this goal's first commit that is not bookkeeping, and stays red until
Stage 6 deletes goal `ci-green`'s record.** That is expected. No session asks for a push because of it,
and no session edits that check to make it pass. Every other floor check is ordinary: red is work.

## Stage 2 — the registers

**Does:** Confirms that every register of owed work names only a milestone that is still ahead.

The registers were closed once already, by goal `gap-zero`. This stage proves they are still closed
after every goal walked since then. Every entry is either owned by a milestone at M9 or later, inside
the scope that milestone's file states, or it is a gap owned by `goal-closeout` and built in Stage 4.

- **The gap register.** `bun nv owners --check` and `bun nv owners --deferrals`. Every gap record under
  `data/gaps/` has a milestone owner whose `docs/plan/<id>.md` covers it.
- **The other registers.** `bun nv owners --registers` counts them: the refusal runs in
  `docs/agent/carried-refusals.md`, the keys in `crates/nvs-stdlib/tests/*-outstanding.txt`, the guard
  names in `docs/agent/guard-name-debt.md`, and the playbook bullets whose `until` names a state of the
  tree. Read each one. An entry that names owed work with no future milestone becomes a gap owned by
  `goal-closeout`.
- **The feature proofs.** `bun nv proofs --gate` owes nothing in the whole roster.
- **The plan.** `docs/implementation-plan.md`'s `Open now` and `Blocking` name nothing owed before M9.
  `bun nv plan --show M0:verify` through `M8:verify` are each met by the tree. A paragraph the tree
  does not meet becomes a gap owned by `goal-closeout`. A paragraph that promises less than the tree
  does is corrected: tested code that is ahead of a document wins, and the document is fixed.

## Stage 3 — every old goal is read

**Does:** Reads every goal in front of this one for unfinished work and for decisions that only its prose holds.

There are about 180 goals. This stage fans out to subagents: each one reads a batch of about fifteen
goals' prose, records and handoff records and reports back. It edits nothing. For each goal it answers
three questions.

1. **Is its work finished?** Compare what its stages promise, its handoff's `backlog`, and every
   `## Known gaps on the path` it names with the tree. Something promised and not built becomes a gap
   record owned by `goal-closeout`, with the `file:line` that owes it.
2. **Does a decision live only here?** A standing decision has a home when a rule fragment, a decision
   record, a module doc, the reference or a tested behaviour states it. A decision with no other home
   gets an entry in `docs/agent/goal-decisions.md`.
3. **Does a check prove something no suite runs?** A `cargo-named` test or an `nvs-suite` case already
   runs in `nv verify`. A `command` or a fixture check may not. One that proves a behaviour no suite
   covers becomes a test or a `.nvst` case in Stage 4.

`docs/agent/goal-decisions.md` opens by saying it is a record for reading and binds no goal, and that a
rule fragment is the only binding form. Each entry says what was decided, where in the tree it applies,
and the slug of the goal it came from, in backticks. A goal with nothing to keep is named in one list
at the end, so every slug appears once. The stage's check fails while any goal in front of this one is
missing from the file.

## Stage 4 — finished in place

**Does:** Builds every gap the audit found, so no gap names this goal.

Each gap owned by `goal-closeout` is built and its record deleted, or struck as a stated bound in its
module's prose, or given a milestone owner whose plan file covers it. That last answer is allowed only
when the work is genuinely that milestone's scope, never to clear the gate. Each missing test from
Stage 3's third question is written. The order is by file set, as always: gaps in one crate are one
group.

Something too large to finish here does not stop the run. "Too large" means it needs a design record
beyond the slots § *Standing decisions* allows, or a decision only the user can make. The session writes
it as a question in § *Questions for the user at the goal end*, and the run goes on with the next gap,
then Stages 5 and 6. A gap waiting on a question keeps its record until the answer is built.

## Stage 5 — deletion becomes the rule

**Does:** Makes the goal switch delete the goal it leaves, and removes the floor machinery that nothing reads any more.

- **The record.** One new decision record, written first. It modifies
  `rule:tooling/the-chain-names-its-live-goal`: a walked goal is deleted at the switch, nothing is
  carried as a floor, and the suites are the safety net. It names this goal as the first goal the rule
  covers, and says that no goal before it was ever deleted by the rule. The fragment is rewritten whole,
  and [README.md](README.md) § *The chain contract* with it, rules 1 and 4 above all.
- **The switch.** `tools/nv/cmd/loop.ts:@advance` deletes the walked goal's prose, record and handoff
  record, removes its slug from `data/chain.json`'s `goals`, renders the goal plan, and commits all of
  it as the switch commit. The commit message keeps its form, ``docs(loop): the chain advances from `a`
  to `b` ``, and its body says the goal was deleted.
- **The floor.** `tools/nv/lib/chain.ts:@goalPlan` carries no floor, since no goal is ever in front of
  the live one. `walkedGoals`, the graduation of walked test checks in `tools/nv/cmd/chain.ts`, the
  retired-goal handling (`retired` in `tools/nv/cmd/owners.ts`, the empty-checks rule), and the floor
  stage in `tools/nv/driver/accept.ts` are deleted where nothing else reads them, with their tests.
  Simplicity is the reason, and the same commit says what each removal was.
- **The goal-end duty.** Before a session writes `DONE`, a decision that only its goal's prose holds is
  moved to its home: a rule, a record, a module doc. [session-prompt.md](../session-prompt.md) and
  [coordinator.md](../coordinator.md) say so, once, where each owns the goal end.
- **The citation check.** `bun nv chain --check` fails on prose that names a goal not on the chain, as
  it already fails on a goal named by its number. Decision records under `docs/decisions/` and
  `data/decisions/` are frozen history and exempt, and so is `docs/agent/goal-decisions.md`. A goal's
  own prose and handoff record are exempt too, because they are deleted with the goal. This file
  names goal `ci-green` after Stage 6 has deleted it, and that is correct.

After this stage the driver deletes this goal when it is reached. Its checks go with it, which is the
rule working as written.

## Stage 6 — the old goals are deleted

**Does:** Deletes every goal in front of this one and every citation of them.

- **The files.** For every slug in front of `goal-closeout`: `docs/agent/goals/<slug>.md`,
  `data/goals/<slug>.json` and its `.handoff.json`. Any tool, test or fixture whose only job was one of
  those goals goes too. `data/chain.json`'s `goals` starts at
  `goal-closeout`. It is edited by hand this one time, because `bun nv chain --remove` refuses a walked
  goal, and then `bun nv render` writes the goal plan.
- **The links first.** Do the deletion and the repair of every link it breaks in one slice, so `nv
  verify` is green at every commit. `bun nv links` names each broken link. The goal citations that are
  plain text come next, in groups by file set. `bun nv chain --check` names each of them.
- **The citers.** `docs/implementation-plan.md`, `docs/plan/*.md`, `docs/agent/*.md`, the playbook,
  module docs in `crates/`, `tools/nv/`, `tests/` and the examples. A sentence that cites a goal for its
  reasoning is rewritten to give the reason, or to cite the rule or record that holds it. A sentence
  that is only history is deleted, because `git log` keeps it.
- **The README.** [README.md](README.md)'s goal table holds `goal-closeout` and `performance-pass`, and
  its prose says nothing about generated goals or retired goals that is no longer true.

## Standing decisions

- **Finish in place, by the user's decision of 2026-10-01.** The audit does the work it finds.
- **One stop, at the end, by the user's decision of 2026-10-03.** No session writes `BLOCKED` for a
  question while other work is left. It writes the question in § *Questions for the user at the goal
  end* and carries on. The check `no question waits for the user` keeps the goal from being reached
  while one is open. The session that finds that check the only red one writes `BLOCKED` and names
  the section. The user answers every question at once, and the next sessions build the answers.
- **The CI check never asks for a push, by the user's decision of 2026-10-03.** Stage 1 already says
  so: the check leaves with goal `ci-green`'s record in Stage 6. A handoff that says a person has to
  push is wrong, and no question is written for it.
- **An enum gets its own runtime tag, by the user's decision of 2026-10-03.** Gap
  `enum-case-that-reached-a-mixed` is closed by spending the tag `rule:enums/representation`
  reserves. A case turned into a tagged value carries the enum tag, so in `mixed` it is always truthy
  (`rule:enums/truthiness`) and is told apart from an `int`. This is one decision record. It rewrites
  `rule:enums/representation` whole, and a statically typed enum still costs nothing.
- **Checked returns stay, by the user's decision of 2026-10-03.** If the unwind canary
  `benches/abi-probe/tests/unwind_unavailable.rs` trips again (a `cranelift-jit` that unwinds
  through JIT frames), checked returns are kept, because coroutine stack switches and visible
  error-path refcount drops still favour them. That is one decision record. It rewrites
  `rule:errors/propagation`'s sentence about unwind tables, and it turns the canary around so it
  records that unwinding works and fails if that changes back. The tree is back on `cranelift` 0.135,
  where the canary is green, so nothing is owed unless a bump trips it.
- **No carried floor, by the user's decision of 2026-10-01.** The suites and `nv verify` protect
  finished work. Stage 3's third question is how a check that proved more than a suite becomes a test.
- **Deletion is the rule from this goal on, never before it.** The record says so. Goals walked before
  this one are deleted by Stage 6 as the one-time cleanup, not by the rule.
- **The driver changes here by the user's decision.** [coordinator.md](../coordinator.md) § *The
  loop's own machinery is a person's to change* is set aside for this goal alone. Its three conditions
  still hold for each commit: `bun nv orient` still produces a pack, `bun nv loop --list` still reads
  the list, and `nv verify` is green.
- **`docs/agent/goal-decisions.md` binds nobody.** It is the home of what it lists, and a later goal
  may decide differently without recording why. A decision a later goal must honour belongs in a rule.
- **Frozen records stay frozen.** A decision record that cites a deleted goal keeps the citation.
- **Tested code ahead of a document wins.** Code that does more than a goal or a plan file promised is
  correct, and the document is fixed. Code that lacks something decided is a gap.
- **No archive.** Nothing deleted is copied anywhere. `git log` is the history.
- **Subagents.** Stage 3 runs one subagent per batch of about fifteen goals. Each is read-only, stays
  under 250k of context, writes any scratch file under `.agent-tmp/` and deletes it before it reports.
  The session writes the gaps and `docs/agent/goal-decisions.md` from their reports.
- **ADR slots:** Stage 5's record, the enum tag's record, and the checked-return record only if the
  canary trips. No other number. Read the next free number right before writing each one, because
  another agent may take a number at the same time. Any other design record is a question in the
  section below.

## Questions for the user at the goal end

Each question is one `### ` heading under this section, written when a session meets it, and is
answered by the user with the others at the goal end. A question says, in order:

- **Gap** — the slug, and the `file:line` that owes the work.
- **What has to be decided** — in two or three sentences someone outside the session can follow.
- **Options** — two to four, each with what it costs in performance, memory, usability and
  simplicity, and what the sessions would build for it.
- **Recommendation** — one option and why, under the priority ordering in `AGENTS.md`.
- **Answer:** `open` — the user replaces `open` with the choice. A session that builds the answer
  deletes the whole question.

The check `no question waits for the user` fails while a line `- **Answer:** open` is left.

### Does a completion file's image show in an editor hover?

- **Gap** — `completion-file-images-are-not-checked-in-an-editor`,
  `data/gaps/nvs-lsp/completion-file-images-are-not-checked-in-an-editor.json`.
- **What has to be decided** — Nobody has looked at it. A loop session cannot open VS Code. The
  hover text already renders, and goal `completion-files` accepted that as the fallback.
- **Options** —
  1. *You check it.* Press F5 in `editors/vscode`, open a project with a `.novis/completion/` file
     whose description has an image link, and hover a completed value. Report "image shows" or
     "text only". It takes a few minutes. The session writes the result into the module doc, and on
     "text only" it states the text as the bound.
  2. *Strike it unchecked.* The module doc says the image was never checked in an editor, and the
     text fallback is the bound. No work, but the docs then promise nothing about images.
- **Recommendation** — 1. It is the only way to know, and it costs a few minutes.
- **Answer:** 1. The user checks it in VS Code by hand. The result is written here as "image shows"
  or "text only" when the user reports it, and a session builds from that.

### May the extension offer completion inside `nvs.toml`?

- **Gap** — `route-name-and-directive-completion-are-unbuilt`, `crates/nvs-lsp/src/completion.rs`
  and `docs/plan/m10.md:208`. Route-name completion is built either way. This question is only the
  directive half.
- **What has to be decided** — `rule:ide/the-extension-claims-nvs-only` says the extension registers
  `.nvs` only. Its reason is not to fight other extensions over `.php`. Directive completion needs a
  provider in a TOML file.
- **Options** —
  1. *A provider for files named `nvs.toml` only.* The extension does not claim the TOML language.
     It adds one completion provider with the selector `**/nvs.toml`, fed by the directive registry.
     VS Code merges it with a TOML extension's completion, so nothing is fought over. The rule is
     rewritten to say so. Cost: one provider and its host test, nothing at run time.
  2. *Drop directive completion.* It is removed from the rule and from M10's acceptance. No work,
     but a user writes `nvs.toml` with no help from the editor.
- **Recommendation** — 1. `nvs.toml` is where most users make their first mistakes, and the
  provider is small.
- **Answer:** 1. One completion provider for files named `nvs.toml` only, and the rule is rewritten to
  say so.

### Are `Core` reference cards held to the plain voice?

- **Gap** — `reference-cards-are-not-held-to-the-plain-voice`, for example
  `crates/nvs-stdlib/src/objset.rs:157` and `crates/nvs-stdlib/src/math.rs:623`.
- **What has to be decided** — A card is what `nvs help`, a hover and the website print for a member,
  so an end user reads it. `AGENTS.md` § *Text an end user reads* does not cover cards today, and
  many cards use words that section forbids ("holds", "drops").
- **Options** —
  1. *Hold cards to it, and rewrite them here.* `rule:core-api/reference-card` points to that
     section, and sessions rewrite every card by file set. The number of cards to rewrite is not
     counted. It may be many sessions.
  2. *Hold cards to it, and rewrite them later.* The rule changes now. The rewrite becomes a gap
     owned by a milestone at M9 or later, and the cards are fixed when their classes are next
     touched.
  3. *Do not hold cards to it.* The rule says cards are exempt. No work, but help, hover and website
     text read in two voices.
- **Recommendation** — 2. The rule is right for every card a reader sees. Rewriting all of them is
  wording work that would delay `performance-pass` without changing any behaviour.
- **Answer:** 2. The rule changes now. The rewrite is a gap owned by a milestone at M9 or later, and a
  card is fixed when its class is next touched. No sweep over every card.

### May file writes walk from a folder handle instead of a path?

- **Gap** — `zip-extraction-creates-a-file-by-path-after-resolving-its-folder`, `place` at
  `crates/nvs-stdlib/src/zip.rs:839` and `capability::create` at
  `crates/nvs-runtime/src/capability.rs:643`.
- **What has to be decided** — `Core\Zip::extract` resolves each folder and then creates the file by
  its path. A folder swapped for a link between the two steps is followed. `capability::create`
  checks the grant on a path and then opens that path again, so every write door has the same
  window. Closing it needs a new kind of door, and a decision record no ADR slot of this goal covers.
- **Options** —
  1. *Handle-relative walk for `Core\Zip::extract` only.* The grant is checked once on the
     destination. Each level is opened from the handle above it and never follows a link: `openat`
     and `mkdirat` with `O_NOFOLLOW` on Unix, `NtCreateFile` with a root handle on Windows. Cost:
     two platform paths in `nvs-runtime`, one decision record. It is faster than today, because no
     level is resolved twice.
  2. *The same walk under every write door.* Option 1, and `Core\IO`'s writes use it too. It closes
     the window everywhere. Cost: a larger change to `capability.rs` and its tests, several sessions.
  3. *Keep the window and state it.* The module doc names it as a bound: an attacker needs write
     access inside the destination while the call runs. No work.
- **Recommendation** — 1 now, and 2 as a gap owned by a later goal. Security comes first in the
  priority ordering, and option 1 closes the case where an archive picks the names.
- **Answer:** 2. The handle-relative walk goes under every write door in this goal: `Core\Zip::extract`
  and every `Core\IO` write. The user allows the one decision record it needs.

### Does `Core\Server` need members for the request's environment?

- **Gap** — `core-server-has-no-request-environment`, `crates/nvs-stdlib/src/server.rs:12`
  and `docs/spec/01-core-library.md:1161`. `traceId()` is built. This question is only the
  environment half.
- **What has to be decided** — Spec § 15 gives `Core\Server` "the request's own environment",
  replacing `$_SERVER`, and names no member. Most of `$_SERVER` already has a home:
  `Core\Request` has the method, path, query, headers, `host`, `scheme` and `clientIp`, and `Core\Env`
  has the environment variables. What has no home is the HTTP version, the server's own address
  and port, and the client's port.
- **Options** —
  1. *No new members.* Spec § 15 says `Core\Request` and `Core\Env` replace `$_SERVER`, and the gap
     closes. No cost in memory or speed, and the class stays small. A program cannot read the HTTP
     version or the ports.
  2. *Add the missing facts.* `Core\Request::protocol()` and the two addresses with their ports, carried
     on the request from the accept loop. About 40 bytes more per request, and one session per
     member with its proofs.
- **Recommendation** — 1. Simplicity comes before footprint, and no rule or example needs the
  missing facts. A later goal can add one member when a program needs it.
- **Answer:** 1. No new members. Spec § 15 says `Core\Request` and `Core\Env` replace `$_SERVER`.

### What type is captured output under a request?

- **Gap** — `an-isolate-result-output-is-typed-string`, the result shape at
  `crates/nvs-types/src/expr/isolate.rs:593` and the builder at `crates/nvs-stdlib/src/script.rs:1326`.
  `Core\Out::capture` has the same problem at `crates/nvs-stdlib/src/out.rs:151`.
- **What has to be decided** — A child spawned while a request is answered writes into an HTML sink, so
  its captured bytes are already escaped. `output` is a `string` today, so `echo $result->output`
  escapes the page a second time. `Core\Cli\Text` does not fix it: `echo` escapes a `Cli\Text` under a
  request on purpose (`writes_raw` at `crates/nvs-runtime/src/helpers.rs:2776`). Only a `Core\Html\Markup` is written
  unchanged there. The checker cannot know which sink will be in force, so a fix needs a static type
  that covers both carriers. `Core\Out::capture` answers `Cli\Text` under every sink, so it escapes a
  page twice the same way. That follows from those two lines, and no test checks it.
- **Options** —
  1. *The sink's carrier at run time, and a union statically.* The result builder and
     `Core\Out::capture` build the carrier of the sink in force (`Ctx::carrier`). Both are typed
     `Core\Html\Markup|Core\Cli\Text`. `echo` writes the value unchanged under either sink. A program
     that wants a `string` narrows with `is` first. Cost: one carrier object per captured result,
     freed with the request, and about eight programs in the tree that read `output` as a `string`
     change. `nvs_stdlib::debug`'s module doc argues against such a union for `render`. Here the value
     really does depend on the sink. One or two sessions.
  2. *`Core\Cli\Text` under every sink, the same as `Core\Out::capture`.* A child's terminal styles
     reach a CLI parent's screen. Under a request the page is still escaped twice. The same programs
     change, to `->text()`. One session.
  3. *Keep `string` and state the bound.* The module doc says that re-echoing captured output in a
     request escapes it again. No work. The page is wrong but safe, because escaping twice cannot
     inject markup.
- **Recommendation** — 1. It is the only option that gives a correct page (priority 2), it keeps the
  escaping guarantee, and its memory cost is small and freed with the request. It settles
  `Core\Out::capture` in the same sessions.
- **Answer:** 1. The sink's carrier at run time and the union statically, for the isolate result and
  `Core\Out::capture` both.

### What may `bindTo`, `bind` and `call` rebind a closure to?

- **Gap** — `a-callable-has-no-bind-bindto-or-call`, the method-call arm at
  `crates/nvs-types/src/expr/calls.rs:111`, which reports `E0477` for every `callable` receiver today.
- **What has to be decided** — A closure that uses `$this` captures it as a field, and its body is
  checked and compiled against the enclosing class (`crates/nvs-ir/src/lower/closure.rs:214`). A
  `callable` type does not say whether the closure uses `$this`, so the checker cannot judge a rebind.
  Rebinding to an object of another class would let the compiled body read that object with the wrong
  layout, which is a memory-safety hole. PHP also takes a second argument, the scope, which grants
  access to another class's private members. Frameworks use that to read private state. The rule
  `rule:types/callable-absorbs-closure` keeps the three names and says nothing about either case.
- **Options** —
  1. *Same class or a subclass, checked at run time, and no scope argument.* A closure that does not
     use `$this` is returned unchanged, as `rule:statements/a-closure-binds-this-only-where-it-uses-it`
     already says. One that does is copied with the new `$this` if the object is an instance of the
     class the body was checked against, and otherwise throws an error. `null` throws for a closure
     that uses `$this`. A scope argument does not compile, and `bind` is written `$fn->bind($obj)`.
     Cost: one class test and one closure copy per call, freed like any object. PHP code that binds to
     an unrelated class or reads private members through a scope stops working. Two or three
     sessions: a runtime helper, the closure class records its `$this` class, the checker arm, the
     lowering, a conformance case, and the feature proofs.
  2. *Option 1, and also check statically.* The closure literal's type records whether it uses
     `$this` and which class, so a rebind that cannot succeed is a compile error. Cost: a new part of
     the `callable` type that every assignment and comparison of callables has to carry. More sessions.
  3. *Only the `$this`-free case.* `bindTo`, `bind` and `call` compile, return or call the closure
     unchanged, and throw an error for a closure that uses `$this`. Cost: one session, but PHP code that
     rebinds a closure to another instance of the same class stops working.
- **Recommendation** — 1. It keeps every rebind memory-safe (priority 1). It runs every PHP rebind
  that stays inside the class the body was written for (priority 2). The scope argument is refused
  because it is a way around `private`, which `rule:types/callable-absorbs-closure` does not promise.
- **Answer:** 1. Same class or a subclass, checked at run time, and no scope argument.
