Human output is the default and is written as the run goes, which is what keeps a long suite
legible. A JUnit XML document and a versioned JSON one are each written whole at the end, because
neither has a prefix worth streaming. All three render the same case list and a verdict is decided
**once**, so the plaintext mark, the JSON string and the XML element cannot disagree about how a
test came out.

Under a machine format the program's own output goes to **stderr**: the redirect that produces the
document makes stdout the document, and a test's `echo` interleaved into it would produce something
no parser accepts. It is not quoted back into the document either — a whole suite's output is a cost
every run would pay for the few being debugged. A machine format also names a `#[Test]` run, so
asking for one over a `.nvst` tree is refused rather than silently ignored: the two share no summary,
so there is no document for it to be about.

The JSON schema is versioned and carries what XML has nowhere to put: a structured diff, per-row
results, a shrunk counterexample and per-test coverage. It is at `schemaVersion: 2`, where every record
says where its test is written — `file`, `line` and `column`, one-based, the same location
`nvs check --json` carries for a diagnostic — because a `class` and a `method` are enough to print a
line and not enough to open a file, and nothing below the compiler can recover the rest. The JUnit and
plaintext renderings are unchanged: JUnit has no version to raise, and a path on every line is noise for
the reader the plaintext one is written for. Under a coverage flag (`rule:testing/coverage-report`) the
document is `schemaVersion: 3`, and every record also carries `coverage`: each file name mapped to the
sorted lines that test reached in any attempt. A run without one writes version 2 unchanged.

**`--list` is the same run's table, without the run.** It answers from the `#[Test]` table the compile
already built, so a program whose tests fail, hang or `exit` lists exactly as a passing one does, and its
document carries one `{class, method, file, line, column}` per call under `listed` — no verdict and no
summary, since a summary of zeros is a report of a run that did not happen and a `tests` array missing
its verdicts would make a consumer tell the two documents apart by a key's absence. `--format junit`,
`--update` and a `.nvst` tree are each refused beside it rather than ignored.
