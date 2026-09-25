- **`Core\Path::split` keeps `.` and `..` as elements, so `join` of its result is not the normal
  form.** Resolving them is `normalize`'s job alone, so the round trip is an identity only *through*
  the normal form and exact only on a path that is already normal. Assert
  `normalize(join(split($p))) == normalize($p)`, never `join(split($p)) == normalize($p)`, which
  fails on ordinary rows. [until: gone crates/nvs-stdlib/src/path.rs:Core\Path::split]
