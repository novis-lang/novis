Whether a printed page of a sheet is tall or wide.

You set it with the option `orientation` of `Sheet::print`:

- `'portrait'` is `Portrait`. The page is taller than it is wide.
- `'landscape'` is `Landscape`. The page is wider than it is tall. It suits a sheet with many columns.

Any other string throws a `LogicError`. Without `orientation`, the file sets none, and Excel uses its
own default.
