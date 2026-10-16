`nvs lsp-test` runs tests of what the language server returns to an editor.

Each test is a case file with the extension `.lspt`. A case has a document, one request such as
`completion` or `hover`, and the expected result as text. `<|>` in the document is the position of
the cursor. The command takes case files or directories, and it reads every `*.lspt` file in a
directory. It prints `N passed, M failed`. The exit status is not zero when a case failed.

The result must be the same as the expected text, byte for byte. Line and column numbers in a
result start at 1, and an empty result is written as `none`.

`--coverage` prints a table in place of the summary line. The table shows which kinds of syntax
each request was tested on.

**Good to know:** the document in a case often has a syntax error. This is intended, because the
code in an editor is unfinished while you type. `nvs test` is a separate command and has its own
count.
