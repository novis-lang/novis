- **A `Core` row's key material is not always a `Secret*` leaf, and a registry walk that assumes it is
  reads two protocol members as taking no key at all.** `Core\Jwt::signObject` declares its key as
  `CoreTy::Instance(Core\Crypto\KeyPair)` and `Core\Signature::sign` carries its ring inside a
  `CoreTy::Shape`, so a predicate matching only `SecretBytes`/`SecretBlob`/`SecretStr` misses both.
  Walk `Shape` and `Options` as well as `Array`, `Union` and `Nullable`, and name the key-carrier
  classes — `registry::tests::no_protocol_class_exposes_a_raw_value_accessor`'s `CARRIERS` is that
  list. [until: gone crates/nvs-stdlib/src/registry.rs:CARRIERS]
