- **A closure whose body is `throw` fails E0401 against a nullable return type.**
  `fn (string $url): ?string => throw new RuntimeError(...)` reports "expected `string|null`, found
  `never`", though a `never` body fits any return type. Write a block body with `throw` in it until
  the checker accepts it. See `crates/nvs-types/src/ty.rs`. `until: test a_never_body_fits_a_nullable_return_type`
