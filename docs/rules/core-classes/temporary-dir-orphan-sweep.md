A process killed outright ran no end-of-script sweep. Its leftovers are reclaimed by the orphan
sweep, which walks the owned root and deletes each entry **whose owning pid is not alive**.

It runs in exactly two places: once at `nvs serve` boot, before traffic, and whenever an operator runs
`nvs tmp clean`, which prints each path it removes and with `--dry-run` deletes nothing. Both read
one predicate, so the two can never disagree about whose entry is whose. It runs on no other
invocation — taxing every CLI start with a root walk to insure against a rare hard kill prices the
common case for the exceptional one. The deliberate consequence is that on a machine where the server
never runs and nobody runs the command, a hard-killed script's directory persists: bounded by crash
frequency, confined to one visible root, one command to clear.

The predicate is owner liveness, **never age**. An age rule is precisely what deletes a long-running
process's files out from under it; liveness cannot, because a live owner's entries are skipped no
matter how old. Every failure mode falls the safe way — a recycled pid makes a dead owner's entry
look alive and it leaks until a later sweep, never the reverse — so the sweep may under-delete and
can never over-delete. There is no force flag that overrides liveness, so the worst the command can
do is nothing.

Like the end-of-script sweep, this one never throws. A deletion the OS refuses is a log line at boot
and a printed line under `nvs tmp clean`, the command still succeeds, and the entry waits for the
next sweep.
