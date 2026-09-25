- **`bun nv splice` and `bun nv session` take a patch in two different formats, and the wrap
  skeleton is the one you will have read most recently.** A splice patch is git conflict markers —
  `--- <path>`, then `<<<<<<< OLD` / `=======` / `>>>>>>> NEW` around each block — while a wrap
  file's `## plan-edit:` is `--- old` / `--- new`, and writing the wrap form into a splice patch is
  refused with a message that names the fix but not the shape. `bun nv splice --help`
  prints the invocation forms and the patch format. [until: gone tools/nv/cmd/splice.ts:<<<<<<< OLD]
