The tree flattens to a single sequence: for each `--config` file in order, that file's own keys, then
its includes depth-first in list order; then the next `--config` file. **A later assignment wins**,
which makes an include override the file that pulled it in — the base-plus-local shape, with the
include line at the point the operator wants overridden.

What replaces the within-file refusal across files is an obligation, not a permission: **every
override is recorded with both origins**, surfaced in the boot log and in full by `nvs config dump
--origin` (`rule:config/check-and-dump-audit-the-tree-offline`). This is the whole of what makes
"later wins" acceptable in a file that grants capabilities; without the record it is the silent
shadowing the format rule refused INI for, and a reader that drops the override list has removed a
security property, not a log line.

The override record is per key. Two files setting different keys of one block both survive, and a
partly written block keeps every unwritten key's default independently.
`rule:config/a-duplicate-key-is-an-error-and-so-is-an-unknown-one` keeps its scope: the same key twice
in *one* file is still refused.

- **Memory:** a live snapshot keeps one origin, a file and a line, for each directive set, plus the
  list of overrides. It is bounded by the number of directives, never by traffic, and the tree is
  parsed at boot and on each reload, never on the request path.

**Declined:**
- **Refusing every cross-file duplicate, so that an include may only add keys no other file set.** A
  host-local file could then never override a base default, only fill a gap the base left open, and
  that removes the reason to split files at all. Recording the override is what makes it safe instead.
