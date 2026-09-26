Returns the extension of a file name: the text after the last dot, without the dot. This replaces
PHP's `pathinfo($path, PATHINFO_EXTENSION)`.

The method reads the last part of the path as text and never looks at the disk. `invoice.pdf` gives
`pdf`, and `backup.tar.gz` gives `gz`. The extension keeps its letter case, so `photo.JPG` gives
`JPG`.

When there is no extension, the result is `null`. A name without a dot has none. A name that starts
with a dot, such as `.gitignore`, has none, because the dot is part of the name. A name that ends
with a dot, such as `report.`, has none too. PHP returns `gitignore` and the empty string for these
two.

The result is exactly what `Core\Path::withExtension` takes, so you can read an extension and write
it back.

**The examples below** read extensions from several paths, show the names that have none, and choose
a content type for a file.
