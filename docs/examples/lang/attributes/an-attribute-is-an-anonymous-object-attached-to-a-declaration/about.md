An attribute attaches fixed values to a declaration. You declare it first, with its name, the places
it may be written and its fields:

```nvs
attribute Table on class { name: string }
```

Then you write it as `#[...]` in front of a class, a method, a property or a parameter:
`#[Table(name: 'users')]`. The words after `on` list where the attribute may be written. Novis checks
your values against the fields. An attribute is not a class. Novis creates no object for it, and none
of your code runs for it.

An attribute may also be declared inside a class or an interface. Then only the classes that extend or
implement that type may use it. Outside that type, you write its name with the owner, as in
`Job::Meta`.

Every value must be known while the program is compiled: a number, a string, `true`, `null`, a list, a
class constant or an enum case. A variable or a function call there does not compile.

**Good to know:** `#` on its own starts a comment. Only `#[` starts an attribute.
