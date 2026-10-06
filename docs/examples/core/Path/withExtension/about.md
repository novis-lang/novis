Returns a path with the extension of its last part changed.

The method works on the text of the path and never looks at the disk. `orders.csv` with `xlsx`
gives `orders.xlsx`. A name without an extension gets one, so `notes` with `txt` gives `notes.txt`.
When the new extension is `null`, the old one is removed.

You write the extension without its dot, the same way `Core\Path::extension` returns it. An
extension may have a dot inside, such as `tar.gz`. An empty extension, one that starts with a dot
and one with a `/` or `\` in it throw a `RuntimeError`. A path with no file name, such as `/` or the
empty string, throws a `RuntimeError` too.

The result is written with `Core\Path::SEPARATOR`, which is `\` on Windows and `/` everywhere else.

**The examples below** change an extension, remove one, and name the converted copy of an uploaded
image.
