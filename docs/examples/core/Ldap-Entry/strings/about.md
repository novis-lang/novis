Returns every value of an attribute as an array of text, such as every `member` of a group or every
group in a person's `memberOf`.

The values are in the order the server sent them. That order is not fixed, so sort the array if you
print it or compare it. If the entry has no value for the attribute, the result is `null`.

`strings` also works for an attribute with one value. The result is then an array with one item.
If one of the values is not UTF-8 text, `strings` throws a `LogicError`.

**Good to know:** every value is `tainted`. Anyone who can change the directory can change this
text, so check or escape each value before you use it in a page, a query or a command.
