The words `ListFormat::join` uses to join the items of a list.

`ListFormat::Type` is declared in the class `Novis\Intl\ListFormat`.

You pass a `ListFormat::Type` as `type`:

- `And` writes "tea, coffee, and water" in English. This is the default.
- `Or` writes "tea, coffee, or water", for a choice.
- `Unit` joins the parts of one amount, such as "5 feet, 11 inches".

The words and the commas come from the locale. German writes "Tee, Kaffee und Wasser", with no comma
before "und".
