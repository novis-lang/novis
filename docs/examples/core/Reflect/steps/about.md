Returns the property that each part of a property path names.

`Core\Reflect::steps` takes a property path, such as `customer.address.city` of type
`propertyPath<Order>`, and returns an array of `Core\Reflect\PropertyInfo`. There is one entry for
each part of the path, in order. Each entry has the property's name, whether it is public, and the
type it is declared with.

The type tells you how the path reaches the next part. A type like `Customer` is one object. A type
like `?Customer` is an object or `null`. A type like `array<Line>` is an array of objects. A library
can use this to build a database join for each part of the path.

**The examples below** show each part of a path, a join built for each part, and a check of a sort
field that a request sends.
