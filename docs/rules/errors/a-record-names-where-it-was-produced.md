A record's envelope carries where it was produced — the file as the program named it, its one-based
line, and the enclosing `Class::member` where there is one — filled by the producer from a constant
the compiler already knows, and a `Throwable`'s `location` is that same datum rather than a second
spelling of it.

Two spellings of "where" that could disagree would be worse than one that was missing, so there is
one construction with two readers. A producer that has no source to give omits the field, on the
envelope's existing rule that an absent field is omitted rather than rendered empty.

**No stack is captured per record.** Where a trace is active the record already carries `span_id`
(`rule:observability/a-log-record-carries-trace-ids-when-a-trace-is-active`) and the trace already
records call entry and exit at every call site, so *how execution arrived* is reconstructable for
exactly the sessions that asked for a trace. Where one is not, the file, line and member are the
answer, and they are enough to open an editor in the right place. Snapshotting the whole stack on
every record would need a walk of Novis's own frame chain and is not bought here.

The cost stays where `rule:errors/propagation` put it. A `source` is read at the call that produces a
record, never maintained as running state, so no path that produces no record pays anything for it.
