A debug record reaches the service by being written to a file the writing core owns alone, never over
a socket, and the service reads a spool file only once it has been sealed.

The application therefore opens no listener and makes no outbound connection for this: there is
nothing on the request path but a buffered append to an already-open descriptor, which is cheaper
than a socket write rather than more expensive. It is never `fsync`ed per record — that, and only
that, is what would make this cost milliseconds. Nothing is lost while the service is stopped,
because the files are simply still there when it starts.

**One file per writer, sealed by an atomic rename** at a size or an age, whichever comes first. Per
writer because the server accepts per core, and `O_APPEND` gives an atomic offset bump and not an
atomic large write, so a shared file would tear records that a dumped object tree easily makes large
enough to tear. Sealed by rename because a reader then never has to ask whether a file is still being
written. Deleted after ingest, so this stream needs no rotation of its own. A file a crash left
unsealed and untouched for longer than any live writer would take is swept by the same pass.

The seal interval is the latency between a dump and its appearing, which is what buys everything
above.
