A `.nvs` file can be a page as well as a program: text outside the tags is written out as it
stands, and code between them runs.

A file starts in **text mode**, where every byte goes straight to the output. `<?nvs` switches to
**code mode**, and `?>` switches back. A file that opens with `<?nvs` and never closes it is simply
a program with no page around it, which is what most files are. `<?= $name ?>` is the short form for
writing one value where the tag stands, and it is what a template uses for nearly every hole in the
page. The two modes may alternate as often as you like, and a `{ … }` block can begin in one and end
in another, so a loop or an `if` can wrap plain HTML without any of it being written as text inside
your code.

**Good to know:** a `?>` followed straight by a newline swallows that newline, so a line that ends
with a closing tag leaves no blank line behind it. `<?php` is refused with a message pointing at
`<?nvs`, and there is no short `<?` tag, so a `<?xml` line in your page stays text.
