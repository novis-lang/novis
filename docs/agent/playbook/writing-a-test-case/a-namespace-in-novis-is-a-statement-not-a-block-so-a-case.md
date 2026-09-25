- **A `namespace` in Novis is a statement, not a block, so a case that needs two of them needs two
  files.** `namespace App { ... }` is `E0243`, and declarations cross a `require`
  (`rule:statements/require-is-the-only-inclusion-construct`), so the shape is a `--FILE app.nvs--`
  plus a `require './app.nvs';` at the top of the root file, whose statements alone print. `bun nv try`
  cannot run it, but `target/debug/nvs test <path>.nvst` takes a single case path and reproduces its
  working directory. [until: gone crates/nvs-diagnostics/src/lib.rs:Code::new("E0243")]
