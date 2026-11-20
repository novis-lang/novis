Returns the full name of the declaration an attribute was written with.

Every attribute has a declaration, such as `attribute Route on class { path: string }`. The name
includes the namespace. In the namespace `Shop`, `#[Route(path: "/orders")]` gives `Shop\Route`.
An attribute declared inside a class also has the class name, such as `Shop\Controller::Cache`.
An attribute declared outside any namespace has its short name, such as `Route`.

`Core\Reflect` gives you one `Core\Reflect\AttributeInfo` for each attribute of a class. Compare
`name` with the name you are looking for to keep one kind of attribute and skip all the others.

**Good to know:** two declarations in different namespaces can have the same short name. The full
name tells them apart.
