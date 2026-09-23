- **A RustCrypto crate at its newest major can be generic over a different `digest` generation than
  `Core\Hash`'s.** `hkdf` 0.13 and `pbkdf2` 0.13 take a `digest 0.11` hash while `sha2.workspace` is
  0.10, so `Hkdf::<sha2::Sha256>` fails an `EagerHash` bound with an error naming
  `CoreWrapper<CtVariableCoreWrapper<…>>` and never the version. Check which `digest` major a crate
  is generic over before pinning it, and name the matching hash through the root manifest's
  `sha2-v11` row rather than downgrading the crate, which would only add a duplicate.
  [until: gone Cargo.toml:sha2-v11]
