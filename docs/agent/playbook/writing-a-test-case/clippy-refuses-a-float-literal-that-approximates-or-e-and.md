- **Clippy refuses a float literal that approximates π or e, and refuses `assert!` over two
  constants.** A compile-time invariant belongs in `const _: () = assert!(…);`, not a `#[test]`.
  [until: gone Cargo.toml:clippy]
