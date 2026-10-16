A `.nvs` file can be a page and a program at once. Text outside the tags is written to the output
unchanged, and code between the tags runs.

A file starts in text mode, where every byte is written to the output. `<?nvs` starts code mode and
`?>` ends it. Most files start with `<?nvs` and never close it, so the whole file is code.

`<?= $name ?>` writes one value at the place where the tag is. A template uses it for most values
in a page. You can change mode as often as you like. A `{ … }` block can start in one code section
and end in a later one, so a loop or an `if` can repeat or skip plain HTML.

**Good to know:** a newline directly after `?>` is not written to the output, so a line that ends
with a closing tag leaves no empty line. `<?php` does not compile, and the error message names
`<?nvs`. There is no short `<?` tag, so a `<?xml` line in a page stays text.
