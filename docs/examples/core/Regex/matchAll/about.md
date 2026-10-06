Finds every place where a pattern matches a text, and returns the matches in an array.

Each item in the array is a `Core\Regex\Match`, in the order the matches appear in the text. A
`Match` has the matched text, the position where it starts, and the text of each group in the
pattern. Matches do not overlap: the next search starts where the previous match ends. If the
pattern matches nowhere, the array is empty. Positions count characters, not bytes.

**Good to know:** to find only the first match, use `Core\Regex::match`. It stops after one match,
so it is faster on a long text.
