There is no ambient session array and no implicit session start. A script calls `Core\Session::start`
— or an equivalent resuming a presented identifier — before reading or writing session state, so
"this request uses sessions" is a line in the source rather than a fact discoverable only by grepping
for a superglobal. A member called before `start` throws naming itself, and the refusal is catchable
at the root like any other.

The shape is fixed here: a class, an explicit start, no ambient array. The mechanics are a feature of
their own — the store is the shared cache tier or the database, the local tier is refused at boot
because a session in per-core memory is not a session, there is no lock, and expiry is the store's
own rather than a sweeper's.

What this costs is one line per request that uses sessions. What it buys is that a request that does
not use them pays nothing, which the ambient version could never promise.
