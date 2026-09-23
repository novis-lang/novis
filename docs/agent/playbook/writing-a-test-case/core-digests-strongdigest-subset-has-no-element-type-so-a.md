- **`Core\Digest`'s `StrongDigest` subset has no element type, so a loop over the digests
  `Core\Hash::hmac` accepts does not compile.** `array<Core\Digest>` iterates to a plain
  `Core\Digest`, which `hmac`'s third parameter refuses with `E0401`, and no spelling names the
  union of the ten case types. A proof that asks all ten spells each call once, in a helper that
  returns what it needs (`tests/hostile/core/Hash/hmac/01-keys-of-every-size.nvs`'s `Every::sizes`).
  [until: reviewed 2026-09-22]
