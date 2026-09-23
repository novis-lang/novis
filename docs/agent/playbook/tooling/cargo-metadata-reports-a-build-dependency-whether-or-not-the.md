- **`cargo metadata` reports a build dependency whether or not the feature that uses it is on, and
  `links` is not a C signal at all.** `python tools/gen-attribution.py --check-c-deps` over the
  resolved graph lists `blake3`'s `cc` build-dependency even under `features = ["pure"]`, and
  `defmt`'s `links` is Cargo's one-version token rather than a native library. An enumeration is a
  **signal** and the ledger entry is the finding: a `no-native-code` verdict resting on a feature
  names it in `C_DEPENDENCIES`'s `requires` column, and the crate's own `Cargo.toml` comment settles
  each in one look. [until: gone tools/gen-attribution.py:C_DEPENDENCIES]
