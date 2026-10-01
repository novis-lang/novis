Library and extension code lives at one of three tiers, and each exists because it is the right
answer for a different class of code:

- **Tier 0 — built-in.** `nvs-stdlib`, compiled into the binary: native speed, direct heap access,
  no boundary. This is where fine-grained primitives go — string, array and arithmetic operations,
  anything whose whole cost is comparable to a call — each a `static` member of a `Core` domain
  class (`rule:classes/no-free-functions-or-constants`).
- **Tier 1 — a sandboxed `.nvsx` component.** The default and recommended path for third-party
  code, and where hostile-bytes parsers go (`rule:packaging/an-extension-is-a-sandboxed-wasm-component`).
  The two first-party components, image and intl, are Tier 1 built into the binary and always
  present under `Novis\` (`rule:packaging/the-first-party-components-are-built-in`); every other
  component is loaded from an `[[extension]]` entry.
- **Tier 2 — statically linked native.** A Rust crate compiled into the `nvs` binary, for
  first-party subsystems that need raw sockets, TLS termination or the heap: the database drivers,
  the regex engine, crypto. Safe because it is safe Rust, and built from source, which is exactly the
  right friction for code that runs unsandboxed.

Which tier a candidate lands at is decided by `rule:core-api/tier-placement`'s six ordered tests,
and the resulting roster is `rule:core-api/tier-roster`. The partition is not PHP's: `ctype` being an
extension while `str_pad` is not tracks 1997 build engineering and nothing worth preserving.
