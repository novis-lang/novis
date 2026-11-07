- **`bun nv nvs-fmt` exits 1 because `packages/rest/` is in no tree of the corpus list.** A package
  added under `packages/` must be named in `tools/data/nvs-fmt-corpus.txt`, or the format check fails
  for a tree nobody changed. Add the package's line there. See `tools/data/nvs-fmt-corpus.txt`. `until: exists tools/data/nvs-fmt-corpus.txt:packages/rest`
