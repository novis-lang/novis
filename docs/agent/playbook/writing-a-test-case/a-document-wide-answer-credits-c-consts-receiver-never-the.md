- **A document-wide answer credits `C::CONST`'s receiver, never the access.** The receiver is a
  `ConstFetch` node of its own, so a diagnostic about the constant — reported at the class expression,
  `crates/nvs-hir/src/members.rs:1058` — fills that cell and leaves `ClassConstAccess` empty. Only a
  position past the receiver's end is inside the access, and a class constant has no token there.
  [until: gone crates/nvs-hir/src/members.rs:ClassConstAccess]
