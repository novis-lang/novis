- **A moved test module also renames its snapshots**, on top of the *Tooling* bullet about `insta`
  snapshots moving with their module: the file name is the test's module path, so
  `parser::tests::foo` becoming `parser::tests::stmt::foo` needs the `.snap` moved and its `source:`
  line updated. Do that by hand instead of accepting the `.new`, and the diff stays a rename rather
  than a delete plus an unreviewable add. [until: gone crates/nvs-ir/Cargo.toml:insta]
