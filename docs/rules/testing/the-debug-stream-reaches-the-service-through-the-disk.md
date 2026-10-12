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

**The records are at rest, so the spool is guarded like a secret.** The spool and its directory are
created for the service's account alone, in a directory that must pass the same ownership check a
configuration file gets. A byte cap with oldest-first eviction is mandatory rather than advisory,
because nothing drains the spool while the service is stopped. Because the files are the transport,
a container needs none: the developer already mounts the project directory, and the same property
makes the service a viewer for a copied-down log directory or a CI artifact.

- **Memory:** one write buffer per writer, held while the writer lives, and no queue anywhere.

**Not on disk.** Nothing writes a spool file and nothing reads one.

**Declined:**
- **A socket collector with the application as its client.** It needs a bounded queue, a background
  drain, a circuit breaker, a retry window, a drop counter, a connect timeout on the request path and
  its own authentication. Every one of those exists only to survive the collector being absent or
  slow, and a file is absent-proof by construction.
- **One file per record.** It costs a create, a write, a close and a later delete per record. On
  Windows real-time antivirus scans each new file on close, and an undrained directory of small files
  takes seconds to enumerate.
