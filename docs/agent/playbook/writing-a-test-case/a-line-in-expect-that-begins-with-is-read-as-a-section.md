- **A line in `--EXPECT--` that begins with `--` is read as a section header, and the case fails as
  *"not a valid case: unknown section"* rather than as a mismatch.** The `.nvst` reader splits on
  `--NAME--` before it compares anything, so a frozen multipart body or a diff-shaped expectation
  ends the `--EXPECT--` block where its first `--` line starts. Normalise it away in the case, as
  `tests/conformance/core/http-client-streams-a-file-as-the-whole-body.nvst` does with
  `Core\Str::replaceAll`, rather than looking for an escape the format does not
  have. [until: gone tests/conformance/core/http-client-streams-a-file-as-the-whole-body.nvst]
