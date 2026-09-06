Whether a connection may take a new statement is a field on the connection, not on the stream: a
connection is busy whether or not its socket is readable, and SQLite must answer the same
`LogicError` with no stream underneath it at all. The four states are `Idle` (the wire is at a
message boundary), `Executing` (a buffered statement is in flight), `Streaming` (rows remain unread,
so a second statement is `rule:core-classes/db-streaming`'s refusal) and `Poisoned` (the wire is
*not* at a known message boundary). It is a plain cell with no atomic and no lock, because a task
never migrates and a connection is owned by one request at a time.

**A poisoned connection is closed, never reset, and never returned to the pool.** The reset is a
security boundary because a connection carrying one request's state into another's is a cross-tenant
leak; a `RESET ALL` written into the middle of an unfinished message is not a reset but a fragment of
one request's protocol stream that the next request will read as its own. Draining first would mean
trusting a length prefix that has already proven untrustworthy. Closing costs one handshake and is
the only answer that is provable.

An abandoned stream is not automatically poison: a driver that can cancel and drain deterministically
returns to `Idle` and pools the connection, and one that cannot poisons it. That choice is per
driver, in the driver.
