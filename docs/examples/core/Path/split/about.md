Splits a path into its parts: the folders and the file name, in order. It replaces PHP's
`explode("/", $path)`, and it reads both `/` and `\` as separators.

The method works on the text of the path and never looks at the disk. `srv/shop/logo.png` gives
three parts: `srv`, `shop` and `logo.png`. The result never has an empty part. A separator that is
repeated, or one at the end, adds nothing. The empty path gives an empty array.

When the path starts at a root, the root is the first part. The root is `/`, a drive such as `C:\`,
or a network share such as `\\server\share\`. It is written with `Core\Path::SEPARATOR`, which is
`\` on Windows and `/` everywhere else. So `/srv/shop` gives `/`, `srv` and `shop`.

Because the root is kept, `Core\Path::join` can put the parts back together into the same path.

**The examples below** split a path, show the root and the empty path, and build the navigation
links for a page from its path.
