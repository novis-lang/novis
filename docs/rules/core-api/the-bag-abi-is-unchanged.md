The three-state bag changes *which constant fills one slot, for one kind of field*, and nothing else. A bag
is still one ABI argument per declared field, no runtime representation of a shape exists, and no helper
learns a new calling convention (`rule:core-api/shape-flattens-at-the-abi`).

The marker reused is the never-written storage state that already exists for definite-initialization
analysis: already defined as distinct from every legal value including null, already costing **zero
additional bytes** because it is one more discriminant on a representation that carries one, and already
non-refcounted, so it raises no ownership question at a call boundary. Reaching it from a call site costs
one constant-argument variant, one instruction constant beside the null one, and the codegen arm that
writes the tag byte.

What it spends (`rule:programs/memory-priority`): **nothing per request and nothing per call** — the
omitting call site emits one constant either way. A non-nullable field is unaffected in every respect, so
every member registered today lowers to the same instructions and nothing needs migrating.

`crates/nvs-runtime/src/value.rs` carries the marker tag, `nvs_ir::ir::InstKind::ConstUnset` is the
one instruction that materializes it, and `nvs_codegen`'s arm for that writes the tag byte over a
zero payload. Every member registered before this lowers byte for byte as it did.
