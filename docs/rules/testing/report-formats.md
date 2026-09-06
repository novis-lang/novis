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
results, a shrunk counterexample and per-test coverage.
