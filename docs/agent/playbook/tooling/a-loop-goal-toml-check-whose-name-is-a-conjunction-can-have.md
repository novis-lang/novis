- **A `loop-goal.toml` check whose name is a conjunction can have its two halves in two *crates*,
  and the tell is that one half names a compile-time fact.** "as tainted" is a qualifier on a
  `nvs_stdlib::registry` row's signature; a crate that names neither `nvs-stdlib` nor `nvs-types`
  can carry the value but has nothing to write the qualifier with. Ask which crate can host each
  half of an "as …" or "and …" name separately, and split the check. [until: gone tools/nv/cmd/orient.ts:a loop-goal.toml*]
