How a value is compared with the values of a rule.

`Sheet::conditional` takes the option `operator` for the kind `'cell'`. The method turns the string
into a `Sheet::Operator` case:

| String | Case | A cell matches when it is |
|---|---|---|
| `'between'` | `Between` | from the first value to the second |
| `'notBetween'` | `NotBetween` | outside the two values |
| `'equal'` | `Equal` | equal to the value |
| `'notEqual'` | `NotEqual` | not equal to the value |
| `'greater'` | `Greater` | greater than the value |
| `'greaterOrEqual'` | `GreaterOrEqual` | greater than or equal to the value |
| `'less'` | `Less` | less than the value |
| `'lessOrEqual'` | `LessOrEqual` | less than or equal to the value |

`Between` and `NotBetween` need two values. The others need one. `Sheet::validate` sets the operator
for you: `min` and `max` give `Between`, `min` alone gives `GreaterOrEqual`, and `max` alone gives
`LessOrEqual`.
