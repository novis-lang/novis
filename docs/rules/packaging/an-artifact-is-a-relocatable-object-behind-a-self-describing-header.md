Every artifact file is a fixed header followed by its payload:

```
magic ("NVSC") | format_version: u16 | env_hash: 32 bytes
  | payload_len: u64 | BLAKE3(payload): 32 bytes | payload
```

`env_hash` is repeated here although it is already folded into the path — defense in depth against a
BLAKE3 collision or a file placed at that path by hand, bought for the cost of one comparison.

**The payload is relocatable code, not an image of pages.** A source file's entry holds one
*piece* per function, what `nvs_codegen::compile_pieces` returns: its bytes, its alignment and its
relocations (`rule:packaging/an-artifact-is-one-immutable-content-addressed-file` gives the three
payloads). Each relocation names its target by what it is: another function by its Novis name, a
literal by its digest, and anything else by its symbol. A piece is the same in every program that
gives it the same key, so the file carries no address and no position the compiling process
invented. Pieces compile under the JIT's ISA flags, `is_pic` off included, so every reference is
an absolute relocation and a warm hit runs the instructions a cold compile runs. The writer
publishes an entry only after its own placer placed the program. What belongs to a process rather
than a program stays a symbol: every runtime helper, every class descriptor as
`nvs_class_desc_<escaped label>`, and each function's statement, edge, query and index base. A
payload cannot run where it lands;
`rule:packaging/an-artifact-is-verified-whole-before-a-page-is-executable` makes it runnable.

**It carries no class metadata at all.** A descriptor is built by the loading process out of the lowered
IR exactly as a cold compile builds one; because the key covers the source and the toolchain, a hit is
by construction a run whose IR is identical to the one the payload came from, so the descriptors this
process allocates are the ones the payload's undefined names refer to. A change to which symbols a
payload leaves undefined, or to what a reader must do with them, is a `format_version` bump: an older
file becomes a plain miss rather than one relocated under yesterday's rules.

**Declined:**
- **A position-independent payload (`is_pic` on).** It reaches every symbol through a GOT cell, one more
  load per reference than the JIT's code. COFF also gives every such cell a section of its own, and
  COFF counts sections in 16 bits: a unit of about 4,000 functions passed 65,535 and never loaded on
  Windows. The placer places and relocates every piece itself, so position independence buys
  nothing here.
- **A host-format object file as the payload.** One per program cannot reuse a function, and one
  per source file needs a module that declares the whole program once per file. A piece is the
  part of an object a placer reads.
- **Class descriptors carried in the payload, so a warm hit skips the front end too.** It serialises a
  runtime type: a `nvs_runtime::ClassDesc` holds strings, codec field lists and pointers to other
  descriptors, so every future field of it becomes a `format_version` bump and a second, write-only
  spelling of a structure the compiler already builds from the IR. What it buys is only the front end,
  which a miss pays anyway (`rule:packaging/a-warm-hit-skips-codegen-not-the-front-end`).
- **A compressed payload.** Decompressing into a fresh buffer would come before every check and every
  placement, spending CPU on each hit to save disk on a payload that is already small, and losing the
  single pass the read path is built around.
