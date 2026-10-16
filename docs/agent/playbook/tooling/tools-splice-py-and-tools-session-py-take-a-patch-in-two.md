- **`bun nv splice` and `bun nv plan --apply` take a patch in two different formats.** A splice
  patch is git conflict markers —
  `--- <path>`, then `<<<<<<< OLD` / `=======` / `>>>>>>> NEW` around each block — while a plan
  file's `## plan-edit:` is `--- old` / `--- new`, and writing the plan form into a splice patch is
  refused with a message that names the fix but not the shape. `bun nv splice --help`
  prints the invocation forms and the patch format. See `tools/nv/cmd/splice.ts`. `until: gone tools/nv/cmd/splice.ts:<<<<<<< OLD`
