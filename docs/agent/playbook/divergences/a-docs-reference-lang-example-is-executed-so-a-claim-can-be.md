- **A `docs/reference/lang/` example is executed, so a claim can be pinned instead of asserted.**
  `tools/reference.py` runs every ` ```nvs ` fence against the binary and checks the ` ```output `
  fence after it; ` ```nvs error ` is the fence for a program that must fail `nvs check`, its
  `output` block a substring of the diagnostic. A chapter edit therefore means `python
  tools/reference.py` to regenerate `docs/novis.md` in the same slice, or the generator's `--check`
  fails the verify. [until: gone tools/reference.py:--check]
