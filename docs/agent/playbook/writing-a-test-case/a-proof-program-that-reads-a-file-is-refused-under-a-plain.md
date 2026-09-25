- **A proof program that reads a file is refused under a plain `nvs run`, and no example, attack or
  bench in the tree had ever opened one.** `Core\IO::open` throws `needs the capability fs.read …
  which is not granted`, because a capability is granted only by an `[[app]] entry = "<path>"` block
  in the repository's own `nvs.toml`. Write one block per proof program before writing the program —
  `nvs.toml`'s `examples/files.nvs` and `examples/capability.nvs` blocks are the two shapes, one
  whole-directory and one narrow. [until: gone crates/nvs-stdlib/src/io.rs:Core\IO::open]
