# Playbook — the traps this repository has actually sprung

Hard-won specifics: things that cost a real session real time, written down so the next one does not
pay again. A bullet is a *trap* — "this looks like it should work and does not, and here is what to do
instead" — in three sentences, ending with the `until: <kind> <arg>` trailer that says what retires it
([conventions.md](conventions.md) § *A playbook bullet*; the kinds are looper's `docs/goal-format.md`
§ *The playbook*). **It is not a changelog.** The session that hit the trap, the stage it was on and what it tried first
are `git log`'s to keep, and a bullet that carries them is charged to every session after it.

**One file per bullet.** The bullets are under [playbook/](playbook/), one directory per section and
one file per bullet, named for the words of its bold lead-in. Grep the directory for a bullet's
words to find it. A new trap is a new file and a retired one is a deleted file, so two sessions that
each add a bullet never edit the same file.

**Append, expire, never reword.** Add a bullet when a trap costs you time — `looper wrap`'s
`## playbook: <section>` is the one way in, and it refuses a bullet past 700 bytes or with no path in
backticks. Edit one when it stops being true. `looper playbook retire` deletes every bullet whose
condition holds, so a trap that dies with a flag, a file or a test declares that and leaves on its own. Never reword a
bullet to say the same thing differently: the churn is the cost this playbook exists to avoid.

**Scope.** A rule that binds every agent goes in [AGENTS.md](../../AGENTS.md). A settled rule lives in
the rulebook under [docs/rules/](../rules/), with a decision record for its reasoning. How a subsystem
works goes in that crate's own module doc comment. The *shape* of something you are about to write is
[conventions.md](conventions.md). What is left — the trap — is the playbook, and `looper orient` hands a
session the bullets it ranks highest for the live goal, and names how many more there are.

## Tooling

## Running things

## Writing a test case

## Splitting a file

## Writing Novis

## Divergences
