- **A case that starts the WebAssembly engine can fail a full `bun nv verify` on Windows with `os error 1455`.**
  The engine reserves a table pool of about 1.25 GiB when it starts. On a machine whose page file is
  nearly used up by the parallel run, that reservation fails with `ExtensionError: the WebAssembly engine
  did not start`, and the case prints nothing. `image-a-qr-code-renders-as-svg.nvst` failed this way once
  and passed alone with `bun nv try`. Run the case alone before you treat it as a defect, and rerun
  verify. See `tests/conformance/novis/image-a-qr-code-renders-as-svg.nvst`. `until: gone tests/conformance/novis/image-a-qr-code-renders-as-svg.nvst`
