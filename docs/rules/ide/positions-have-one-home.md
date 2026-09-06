Position encoding is negotiated per LSP 3.17: the server offers `utf-8` and `utf-16`, takes `utf-8` when
the client's `general.positionEncodings` offers it, and falls back to `utf-16`, which is what VS Code sends
today. `SourceFile::line_col` counts `char`s, which is neither, so `nvs-diagnostics` gains `utf16_col(pos)`
and `offset_of(line, col, encoding)` beside it. Position arithmetic has one home and this is it; getting it
wrong is invisible on ASCII and puts every diagnostic on the wrong column the moment a file holds a `ß`.

A document is UTF-8, which is already the language's rule (`rule:types/bytes`). A buffer that is not valid
UTF-8 gets one diagnostic and no further analysis rather than a panic further in. A leading BOM is skipped
and counted, so every offset after it still lands. CRLF is preserved exactly as the document sent it —
spans are byte offsets, so normalising line endings server-side would shift every column in the file, and
the document store is the one place that could happen.
