Only a plain `=` assigns a property in the constructor. `$this->n += 1;` reads the property first, so
a constructor with only that line is the error `E0409`. A `?T` property with no default and no
assignment starts as `null`.

A `return` in a constructor has no value. `return 1;` is the error `E0248`. A bare `return;` leaves
the constructor early, and every property must already have its value on that path. If one does
not, it is `E0409`.

A `lateinit` property may have any type, and a default too. If a method reads it before any write on
the same path, the read does not compile (`E0428`). Any other read with no value first throws a
`LogicError` when it runs. Example 06 prints its message. `isinit` works only on a `lateinit` property. On any other property it is the error
`E0853`. A property cannot be both `lateinit` and `readonly` (`E0427`).

A `readonly` property has no default, and that includes a `static readonly` one (`E0249`). Every
write after the constructor is the error `E0782`: `=`, a compound operator such as `+=`, and an
append with `[]`. A write in another method of the class is `E0782` too, and so is a write in a
subclass constructor after `parent::constructor()`. A second write on the same path through the
constructor is `E0782` as well. `unset($a->v)` is the error `E0413` for every declared property.

A `static` property with no default and a type that is not nullable is the error `E0409`. A
subclass shares the same value with its parent: `B::$n = 5;` also changes `A::$n`. `static::$n`
is the error `E0499`. Write `self::$n` or the class name. `++`, `--` and every compound operator
work on a `static` property. An unknown one is the error `E0309`, and a `private` one read outside
its class is `E0471`.
