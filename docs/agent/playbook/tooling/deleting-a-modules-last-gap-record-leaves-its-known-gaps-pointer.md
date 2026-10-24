- **Deleting a module's last gap record leaves its `# Known gaps` pointer behind, and only the gate's
  `owners` step finds it.** The pointer line names `bun nv gaps --module <file>`, and once no record
  names the module, `bun nv owners --check` fails the block. A build and clippy pass, so the goal's
  last session pays a whole gate run for it. When you delete a record, run `bun nv gaps --module
  <file>`, and if it lists nothing, delete the module's `# Known gaps` heading and its line in the
  same commit. See `tools/nv/cmd/owners.ts`. `until: gone tools/nv/cmd/owners.ts:the one pointer line to its own module's records`
