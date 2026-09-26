Returns the last part of a path: the file name, or the name of the last folder. This replaces PHP's
`basename`.

The method reads the path as text and never looks at the disk. It accepts `/` and `\` as separators
on every platform, so `C:\reports\may.pdf` and `/srv/reports/may.pdf` both give `may.pdf`. A
separator at the end is ignored, so `/srv/reports/` gives `reports`.

A path that is only a root, such as `/` or `C:\`, has no name. The result is the empty string.

With the option `withoutExtension: true`, the method also removes the extension, so `may.pdf` gives
`may`. A name that starts with a dot, such as `.gitignore`, keeps its whole name.

**The examples below** read names from several kinds of path, remove an extension, and take the
file name from an uploaded file.
