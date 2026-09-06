Every artifact file is a fixed header followed by its payload:

```
magic ("NVSC") | format_version: u16 | env_hash: 32 bytes
  | payload_len: u64 | BLAKE3(payload): 32 bytes | payload
```

`env_hash` is repeated here although it is already folded into the path — defense in depth against a
BLAKE3 collision or a file placed at that path by hand, bought for the cost of one comparison.

**The payload is one host-format relocatable object per unit**, what `nvs_codegen::compile_object`
writes, not an image of the pages the JIT would have produced. It is the same `nvs_ir::Program`, walked
by the same lowering, through a second `Module` that records a relocation everywhere the JIT resolves
an address — so the two paths keep one semantics and the file carries no address the compiling process
invented. It defines the unit's own functions, one per IR function and named from its label, and leaves
**undefined** exactly what belongs to a process rather than a program: every runtime helper, and every
class descriptor as `nvs_class_desc_<escaped label>`. A payload cannot run where it lands;
`rule:packaging/an-artifact-is-verified-whole-before-a-page-is-executable` makes it runnable.

**It carries no class metadata at all.** A descriptor is built by the loading process out of the lowered
IR exactly as a cold compile builds one; because the key covers the source and the toolchain, a hit is
by construction a run whose IR is identical to the one the payload came from, so the descriptors this
process allocates are the ones the payload's undefined names refer to. A change to which symbols a
payload leaves undefined, or to what a reader must do with them, is a `format_version` bump: an older
file becomes a plain miss rather than one relocated under yesterday's rules.
