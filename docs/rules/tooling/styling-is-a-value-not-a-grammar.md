The library's grammar count is fixed at four — a regex pattern, a `printf` template, a CLDR date
pattern and a `pack` format (`rule:core-api/shape-rules` R11, each a sink under
`rule:security/every-grammar-is-a-sink`). Both obvious styling designs would be a fifth: raw
`"\e[31m"` escapes, and a `"<red>…</red>"` markup string, which would also collide with the
inline-HTML lexer mode. So styling is typed values, and the count stays four.

```nvs
var $warn = Cli\Style::of({color: Cli\Color::YELLOW, bold: true});
Cli::write(Cli\Text::styled("deleting ", $warn) + Cli\Text::plain($path));
```

`Cli\Style::of({color?, background?, bold?, dim?, italic?, underline?, strikethrough?})` is the one
trailing shape. **`Cli\Color` is a value type, not an enum**: a closed named integer type
(`rule:enums/closed-integer-type`) does not fit a set with sixteen million members. The sixteen named
colours are class constants — `Color::RED` *is* `Color::index(1)`, written shorter, inlined at every
use site as any scalar constant is — and `Color::rgb(uint, uint, uint)` and `Color::index(uint)`
construct the rest.
