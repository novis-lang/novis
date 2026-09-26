Joins a path and any number of parts into one path, with one separator between each part. It
replaces the `$folder . "/" . $file` that PHP programs write by hand.

The method works on the text of the paths and never looks at the disk. `/srv/shop` joined with
`uploads` and `logo.png` gives `/srv/shop/uploads/logo.png`. Extra separators are removed, so
`/srv/shop/` and `uploads` give the same result as `/srv/shop` and `uploads`.

Only the first path decides where the result starts. A later part that starts with `/`, or with a
drive such as `C:\`, is added after the first path and does not replace it. So `/srv/shop` joined
with `/etc/passwd` gives `/srv/shop/etc/passwd`.

`join` does not remove `..`. A part that is `..` stays in the result, and it still means the parent
folder. Use `Core\Path::normalize` to resolve it.

The result is written with `Core\Path::SEPARATOR`, which is `\` on Windows and `/` everywhere else.

**The examples below** join a folder and a file, show what happens to a part that starts with `/`,
and build the path of a customer's upload.
