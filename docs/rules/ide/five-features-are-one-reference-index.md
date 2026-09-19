`nvs-lsp` builds one workspace symbol index, and five features are each one query against it — none
gets a walk of its own. `textDocument/references` is the index's read side. `textDocument/documentHighlight`
is the same query narrowed to the open file, which is why it waits for the index rather than shipping
earlier: it needs resolution applied to every occurrence, not to one. CodeLens above a declaration shows
the reference count, a type's implementors, and the methods a method overrides and is overridden by; it is
gated on `nvs.codeLens.enable` (default `true`), because a lens is a request per visible declaration and a
large file is where it is least welcome. `textDocument/typeHierarchy` shows supertypes and subtypes,
including `rule:classes/interface-default-methods` and `rule:classes/delegation-by-field`, which is
where a reader most needs to see the shape rather than reconstruct it.

The fifth is **unused-member dimming**: a private member, constant or `use` with no reference anywhere in
the index is a diagnostic carrying LSP's `Unnecessary` tag, rendered as dimming rather than a squiggle.
It is only correct at workspace scope — a symbol unused in the open buffer is not unused — so it is
silent under the default of `rule:ide/check-scope-defaults-to-the-workspace` rather than wrong.

Call hierarchy is deliberately not in this list. `textDocument/callHierarchy` is a different index —
call-site edges kept incrementally — and nothing else needs it, so it is not built.

The structural check is that `nvs-lsp` has exactly one symbol-index construction site and all five
readers read it.
