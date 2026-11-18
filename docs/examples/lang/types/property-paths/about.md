A property path is a value that names a property across related objects, such as `customer.address.city`.

The type `propertyPath<Order>` contains paths that start at the class `Order`. Each name in a path is a
public property of the class that the name before it reaches. That property can be an object, a
nullable object or an array. A string written in a call to a `propertyPath<Order>`
parameter is checked when the program compiles. `as` checks any other string while the program runs,
and throws an error if a name is wrong. `as ?propertyPath<Order>` gives `null` instead.

After an array property, `@key` is an element's key and `@value` is the element, as in
`authors.@value.name`. A property named `key` or `value` keeps its name: `authors.value`.

**Good to know:** a `private` or `protected` property is never part of a path. A path can come back to
the same class, such as `parent.parent.name`.

**The examples below** go from an order to its customer and lines, sort by a property the web
address names, show the errors for a wrong path, and use `@key` and `@value`.
