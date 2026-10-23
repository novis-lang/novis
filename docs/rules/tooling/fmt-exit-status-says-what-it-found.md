`nvs fmt` exits `0` when it found nothing to report, `1` when `--check` or `--diff` found a file that
would change, and `2` when a file was refused: it does not parse, or it could not be read or written.
When one run finds both, it exits `2`. `--check` beside `--stdin` reads standard input, writes nothing
and exits by the same codes.

A CI job or a pre-commit hook acts differently on the two failures. A file that would change is fixed
by running `nvs fmt`. A file that does not parse is not, and running the formatter again only repeats
the refusal. Prettier and Black split their exit codes the same way, so a hook written for either reads
this one without a lookup.

**Declined:**
- **One non-zero code for both failures.** A hook then reports a broken file as unformatted, and the
  fix it suggests does nothing.
