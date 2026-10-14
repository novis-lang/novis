Writes the rows of some items of one list in a template.

`Engine::templateItems` takes a `Filling` and a batch. The batch has the `name` of a list, such as
`items`, and its items. Each item is a list of values, and the path of a value is its path inside
the item, such as `qty` for the placeholder `{{items.qty}}`. Each item gets one row, in order. The
result is the bytes of the file that are ready to send.

Call it after `Engine::templateFill` and before `Engine::templateFinish`, once for each batch. Any
other time throws a `LogicError`. So does an item with no value for a placeholder of the row, and a
list longer than the sheet has rows. A list name the template does not have writes nothing.

**Good to know:** most programs do not call `Engine::templateItems` directly. `Template::fill`
returns an `Export`, and when it is written, it calls this for 1,000 items at a time.
