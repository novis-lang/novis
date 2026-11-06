The call a program reaches for first is the safest and fastest way to do the job.

A `Core` member takes the value the program holds and decides itself how much of it to read. No fast path
needs an extra call, a count or a step the caller must remember: a program that hands `Core\Mime::detect`
an upload part reads the part's first bytes, and one that hands `Core\Hash::of` an open file reads it in
pieces, without either program asking for that. A member that needed the caller to read a prefix first,
or to pick a streaming twin, would make the fast and safe path depend on the caller knowing a fact about
the member. That is the defect `rule:http-server/an-upload-is-received-only-through-files` names: an API
whose safe path is selected from a fact the caller cannot see is a defect, whatever the unsafe path costs.

**Where a slower way cannot be removed, it stops at a bound, and its message or its about page names the
direct call.** A random read of an upload part holds the part, so it stops at `[limits] request_body`, and
the error names `saveTo`, a `Core\IO\File` and `readAll({ max })`. The slower way is never the one a
program meets first and never fails silently.

Which values a member that reads bytes takes is
`rule:core-api/a-member-takes-the-source-the-program-holds`.

**Declined:**
- **A second member for the fast path**, such as a `detectFile` beside `detect`. It is a twin the caller
  must know about, and one operation is reachable one way (`rule:core-api/one-paradigm-per-operation`). And the member already knows which source it
  was given.
- **A size hint the caller passes.** The caller cannot know the size of an upload before it is read, and
  a member that trusted a hint would read what the hint said.
