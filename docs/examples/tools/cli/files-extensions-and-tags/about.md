A Novis program is a file with the extension `.nvs` that starts its code with `<?nvs`.

The extension is a convention. `nvs run` accepts any path and reads only the content of the file.
`nvs test` is the one command that uses the extension. It uses it to choose between its two kinds
of test.

Novis does not run PHP. A file that starts with `<?php` does not compile, whatever its name is. The
error is `E0229`.

A file may start with a `#!` line. Novis skips that line and reads the lines after it as code. With
`#!/usr/bin/env nvs` as the first line, you can run the file as a command on Linux and macOS.

**Good to know:** `<?` alone is not a tag. Novis copies it to the output as text.
