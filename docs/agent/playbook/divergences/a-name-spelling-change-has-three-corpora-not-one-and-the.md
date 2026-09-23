- **A name-spelling change has three corpora, not one, and the third only fails at the very end.**
  `.nvs`/`.nvst` fixtures hold the spelling literally, Rust fixtures hold it escaped
  (`"#[\\Core\\Route]"`, two bytes per separator), and a few hold it in a raw string
  (`r#"#[\Core\Route]"#`, one byte), so a regex for either of the first two matches nothing in the
  third and one test in one crate fails last. Sweep for the single-byte form after the escaped one
  and check the hits are only doc comments. [until: reviewed 2026-09-06]
