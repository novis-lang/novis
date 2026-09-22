`--EXPECT--` is exact and frozen, on the same terms as `.nvst`'s: the case's *source* may be corrected
freely, its expectation may not be edited to make it pass.

The rendering is canonical and has one home, `nvs_lsp::render`, so no case invents its own spelling:
diagnostics as `L:C-L:C severity CODE message` sorted by position; a hover as its markdown verbatim; a
definition as `file:L:C` or `none`; completion as the row an editor shows — the label with what is written
directly after it, the kind, the text at the right (`rule:ide/a-completion-row-reads-as-a-declaration`) —
sorted by label; semantic tokens as
`L:C+len type modifiers`; symbols as an indented outline.
