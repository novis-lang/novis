The compiler passes each `Core\Db` member that sends a statement one more constant argument: the call
site's id, an index into the program's query site table. The table maps an id to the file and line of
the call and the hash of that file's text. The id is passed in every mode, so a program's compiled code
is the same whether tracing is on or off. The member reads the id only when its span is recorded, and
the span then carries the table's row for it: the file, the line and the hash.

The host builds the table once per compiled program and gives it to every context that runs the
program, a task of a request included. A recorded span carries the row itself, not the id, so a reader
of the spool or the debug index needs no table of its own. An id is only meaningful in the program
that numbered it, and a recompile numbers the sites again. A context that was given no table, such as
a unit test's, records no site.

The id follows the rosters the registry already keeps for constants of a call
(`rule:security/reflection-enforces-visibility`'s call site is one): there is no argument position a
program can write it in. A call through a callable reference passes the zero id, which names no site,
because the statement goes out wherever the callable is invoked.

The site is exact because the compiler wrote it. Two calls with the same SQL are two sites, and one
call that builds its SQL at run time is one site. The span still holds no bound parameter
(`rule:observability/a-query-is-a-trace-event`): a site id is not a value of the program.

- **Memory:** one integer constant per query call site in the compiled code, and one table row per
  site per compiled program, with one name and hash per file that has a site. A recorded span holds
  the file, the line and the hash. A call nobody traces reads nothing.

**Declined:**
- **Passing the id only when tracing is on.** One compiled program serves requests with tracing on and
  off, so its code cannot depend on either.
- **Grouping queries by their SQL text.** Two sites that run the same statement would read as one.
- **Recording the id, and publishing the table beside the spool.** A reader would join three
  things, keyed by a program hash every record would also need, and a table file the spool's cap
  deletes would leave its records unreadable.
- **Walking the stack to find the caller.** It costs a walk per query, and outside the debug switch a
  frame does not say where it is.
