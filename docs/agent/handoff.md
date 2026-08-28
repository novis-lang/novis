# Handoff

## State

**M4's Stage 8, depth.** The tree is at **791 conformance plus 189 differential**. `python
tools/gaps.py --coverage` ranks by the median cases per member; `Core\Regex` left the frontier this
session — its three thinnest members (`quote`, `replaceWith`, `compile`) each gained a depth case, so
the class's floor is 2 and `Core\Uri` is now the thinnest class named there. Nothing is blocked.

- **`regex-quote-escapes-eighteen-characters-and-nothing-else.nvst`** names the escape set on both
  sides — 18 of the 95 printable ASCII characters gain a backslash, the other 77 are the identity, and
  all 18 are a backslash followed by the character itself, so a member rewriting one into some other
  escape fails while still round-tripping. What the escaping buys is then counted over eight rows and
  three questions each (the unquoted pattern reaches its decoy, the quoted one matches its literal, the
  quoted one no longer reaches the decoy), and four literals that are not valid patterns at all are the
  stronger half. The edges are the empty literal (quoting is the empty pattern, which matches every
  subject at 0) and a literal that is nothing but the 18 (36 characters, still a search and not an
  anchored equality). The differential case deliberately names no set; this one does.
- **`regex-replace-with-s-callback-sees-the-original-subject-and-runs-once-per-replacement.nvst`**
  pins what the callback is *handed* rather than what the member answers: a whole `Match`, whose
  grapheme offset indexes the original subject and not the output built so far — counted with
  `Core\Str::at` over replacements much longer than their matches, which is the drift an implementation
  stepping its own buffer would show. A captured counter object then puts the call count on both sides
  of the `limit` bound (0, 1, 2, 3 and 9 over three matches), and the zero-width pattern is four calls
  over a three-character subject.
- **`regex-compile-answers-a-pattern-every-member-reads-as-its-source-string.nvst`** is the agreement
  shape over the six `Pattern|string` rows: 36 answers, six questions × six patterns, asserting that
  the two spellings agree rather than what either answered. The four options are then checked against
  their inline flag groups (`(?i)`, `(?m)`, `(?s)`, `(?U)`), and the one deliberate difference is named
  — `compile` refuses an impossible pattern at the call that spelled it, the string spelling at
  whichever member first uses it.

## Next group

**`Core\Uri`'s floor** -- `gaps.py --coverage` now ranks it thinnest at median 3.0 with a floor of 1.
The file set is `crates/nvs-stdlib/src/uri.rs` (its `CLASS` roster runs from `uri.rs:347`) plus
`tests/conformance/core/`. Run `python tools/gaps.py --coverage` first, to take the ranking off the
tree rather than off this line.

- [ ] **The four encoders round-trip, and their two sets differ** (`uri.rs:361` `encodeComponent`,
      `:368` `decodeComponent`, `:375` `encodeFormValue`, `:382` `decodeFormValue`) -- one case. The
      *invariance over a sweep* shape: `decode(encode($s)) == $s` counted over a table of reserved
      characters, plus the boundary the two pairs disagree on, which is the space (`%20` against `+`).
- [ ] **`with` agrees with the seven component readers** (`uri.rs:461` the row, `:405`-`:454` the
      readers) -- one case. The *agreement* shape: after `with({host: ...})` every reader answers what
      it answered before except the one named, counted over a sweep rather than read off a line.
- [ ] **`resolve` and `compareTo`'s bounds** (`uri.rs:499`, `:506`) -- one case. RFC 3986 § 5.4's
      reference-resolution rows on both sides, and `compareTo` as the one content comparison two URIs
      have -- ADR 0090 § 4 forbids `==` for it, which the playbook's `loop-goal.toml` bullet already
      names as a comment that got this wrong once.

## Backlog

- `Core\Regex`'s `split` options and `match`'s `{from:}` are the next depth after this group —
  `crates/nvs-stdlib/src/regex.rs`.
- 54 of the 156 guard tests `loop-goal.toml` names match nothing `cargo test` runs —
  `docs/agent/guard-name-debt.md`.
- `nvs-stdlib` gap 3: `matchAll` converts each offset over the subject's prefix, O(n·k) —
  `crates/nvs-stdlib/src/regex.rs` module doc, *Known gaps*.
- ADR 0056 § 4's pattern sink is not enforced, `tainted` having no `Core`-facing half — same module
  doc, gap 1.
