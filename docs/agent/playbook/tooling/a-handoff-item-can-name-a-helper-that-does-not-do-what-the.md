- **A handoff item can name a helper that does not do what the item says it does.** This session's
  item named `nvs_lsp::definition`'s `named_at` as "what already finds the name inside one"; it finds
  the innermost *node* at an offset and hands back that node's whole span, and
  `nvs_syntax::walk` modelled no name span at all — `crates/nvs-lsp/src/redactions.rs`'s module doc
  is where that was written down, two crates from the item. Read the named anchor's own doc comment
  before designing around it: an item is written by a session that had the file open and is
  paraphrasing from memory, so it is a pointer and not a specification. [until: gone tools/nv/cmd/orient.ts:YOUR ITEM]
