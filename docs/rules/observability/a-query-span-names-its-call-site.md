The compiler passes each `Core\Db` member that sends a statement one more constant argument: the call
site's id, an index into the program's query site table. The table maps an id to the file and line of
the call and the hash of that file's text. The id is passed in every mode, so a program's compiled code
is the same whether tracing is on or off. The member reads the id only when its span is recorded, and
the span then carries the file and line it names.

The id follows the rosters the registry already keeps for constants of a call
(`rule:security/reflection-enforces-visibility`'s call site is one): there is no argument position a
program can write it in. A call through a callable reference passes the zero id, which names no site,
because the statement goes out wherever the callable is invoked.

The site is exact because the compiler wrote it. Two calls with the same SQL are two sites, and one
call that builds its SQL at run time is one site. The span still holds no bound parameter
(`rule:observability/a-query-is-a-trace-event`): a site id is not a value of the program.

- **Memory:** one integer constant per query call site in the compiled code, and one table row per
  site per compiled program. A recorded span holds one more integer. A call nobody traces reads
  nothing.

**Declined:**
- **Passing the id only when tracing is on.** One compiled program serves requests with tracing on and
  off, so its code cannot depend on either.
- **Grouping queries by their SQL text.** Two sites that run the same statement would read as one.
- **Walking the stack to find the caller.** It costs a walk per query, and outside the debug switch a
  frame does not say where it is.
