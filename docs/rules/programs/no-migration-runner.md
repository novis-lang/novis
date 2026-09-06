`Web\Migration` has a place in the package roster and **no semantics**. Each of these is undecided:
ordering and dependency between migrations; transactional DDL where the backend supports it, and what
happens on the backends that do not; locking, so two instances of a fleet cannot run the same migration
twice; what "reversible" means, and whether a down-migration exists at all; and how any of it is safe
against a live multi-tenant database.

Every one of those interacts with taint tracking — DDL is a sink — with the database API, and with
root-owned configuration, and none of them is obvious.

**This is a known gap, not an oversight.** Until it is closed by a decision of its own, nothing in
`nvs/web` may ship a migration runner, and one appearing in the package without that decision is a
review failure.
