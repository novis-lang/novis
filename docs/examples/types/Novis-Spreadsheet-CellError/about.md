The error that a cell in a spreadsheet file shows.

When a formula fails in Excel, its cell shows an error such as `#DIV/0!`. `Reader::rows` gives the
value of that cell as a `CellError` case:

| Case | Excel shows |
|---|---|
| `Null` | `#NULL!` |
| `DivZero` | `#DIV/0!` |
| `Value` | `#VALUE!` |
| `Ref` | `#REF!` |
| `Name` | `#NAME?` |
| `Num` | `#NUM!` |
| `NotAvailable` | `#N/A` |
| `GettingData` | `#GETTING_DATA` |
| `Spill` | `#SPILL!` |
| `Calc` | `#CALC!` |

Test a value with `$value is CellError` before you use it as a number or a string.
