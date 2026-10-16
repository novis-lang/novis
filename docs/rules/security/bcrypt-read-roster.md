Verification accepts exactly two stored shapes: the modern hash this library writes, and a bcrypt hash
under the `$2y$`, `$2a$` or `$2b$` prefix — one algorithm under three tags, verified identically. **The
roster grows only by amending this rule, never by accepting what a parser happens to read.**

`$2x$` is refused: it exists to be bug-compatible with `crypt_blowfish`'s sign-extension overflow, and
verifying it means reimplementing the bug.

bcrypt is on the roster because it is what a real population's column holds. PHP's `PASSWORD_DEFAULT`
has written bcrypt since `password_hash` appeared, and Argon2 never became the default, so the user
table of almost every migrating application is a bcrypt column. Without a read path that population
would catch the unreadable-hash throw around every verification, which launders the storage-bug signal
the throw exists to raise (`rule:security/an-unreadable-stored-hash-throws`), or build a dual-column
scheme by hand.

This is what lets an existing user table, hashed by another runtime, verify on day one and rewrite itself one
successful login at a time (`rule:security/needs-rehash-answers-weaker`). The reader is a pure-Rust
bcrypt crate with a constant-time comparison of the digest.

- **Memory:** about 4 KiB, the eksblowfish key schedule, held transiently per verification of a legacy
  row on the calling task. It is bounded by in-flight logins, and shrinks as the table converges.

**Declined:**
- **Reading every algorithm a verifier library can read.** Each entry is a parser, an attack surface and
  a promise kept forever, and two shapes cover the migrating population. A third shape is added the way
  bcrypt was: by being what a real population's column holds.
- **An offline tool that converts the column.** A stored hash is not invertible, so only a presented
  password can be rehashed, and the login-time loop is the only migration path.
- **A bcrypt write path.** No new hash should ever be bcrypt (`rule:security/hash-writes-argon2id-only`).
