- **A `Core\Db` statement written as a literal has its placeholders checked while compiling, so an
  attack that writes the mismatch inline never reaches the runtime it was written for.** `E0770`
  reads a constant query with the same rewriter the request would have used, and a hostile case
  fails on any compile diagnostic, so `$db->query('… = ? or … = ?', ['one'])` is a refusal before
  the program starts rather than the `LogicError` the call makes. Build the statement from a value
  the compiler cannot read — `'… = ' . Core\Str::repeat('?', 1)` — and the check the call itself
  makes is the one that runs. [until: reviewed 2026-09-21]
