- **`Edit` matches `old_string` as a substring, so leading whitespace in the anchor does not pin the
  indent — it matches every line indented at least that far.** Adding a field beside `public: true,`
  in 22 files, the pass anchored at twelve spaces also hit the twenty-space copies in
  `crates/nvs-runtime/src/object.rs`, and the follow-up pass anchored at twenty spaces inserted the
  field there a second time; `cargo build` caught it as *field specified more than once*, and
  nothing before that did. For a repeated one-line insertion write the whole run as a
  `bun nv splice --patch` file, which matches every block before it writes a byte.
  [until: gone tools/nv/cmd/splice.ts:<<<<<<< OLD]
