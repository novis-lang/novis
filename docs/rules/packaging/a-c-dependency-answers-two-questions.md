"Pure Rust by default, deviations argued individually" is a standing test rather than a case-by-case
argument, so the answer does not depend on who argues it:

1. **Does attacker-controlled data reach this code?** If no — accept it under ordinary audit.
2. If yes — accept it only with a **demonstrable, exceptional verification record**. Otherwise it
   must be confined to wasm.

SQLite passes the second question: its test suite is orders of magnitude larger than its source and it
is continuously fuzzed. Almost nothing else clears that bar, which is the point. A codec, an archive
reader or an XSLT engine fails it and is therefore Tier 1 — including when the only implementation is
C, since compiling a C library to wasm is the standing answer when the test fails; lossy WebP encoding
via libwebp is admitted exactly that way. An authentication handshake handles attacker-reachable bytes,
so a driver plugin the pure-Rust crates lack gets a Rust implementation or a documented refusal, never a
C dependency.

The test is enforced rather than remembered: `bun nv gen-attribution --check-c-deps` enumerates the
default binary's C dependencies from the resolved graph and fails CI on any without a recorded answer
(`rule:testing/attribution-is-diffed-in-ci`).
