Fills the placeholders of a template that are not in a list.

`Engine::templateFill` takes a `Filling`, a list of values and the names of the lists. Each value
has a `path`, such as `customer.name`, and the value for the placeholder `{{customer.name}}`. A
list name such as `items` marks the placeholders `{{items.name}}` and `{{items.qty}}` as a row
that repeats once for each item. The result has `data`, the first bytes of the file, and `lists`,
the lists the template uses in the order of their rows.

A placeholder with no value throws a `LogicError`. A second call throws a `LogicError`, and so
does a list name given twice. A value with a path that no placeholder has is not written, and a
list the template does not have is not in `lists`.

**Good to know:** most programs do not call `Engine::templateFill` directly. `Template::fill`
turns the data of the program into paths and calls it once.
