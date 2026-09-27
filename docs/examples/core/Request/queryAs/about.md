Reads the query string of the request into a class or a shape that you write. The query string is
the part of the address after the `?`, such as `q=shoes&page=2`. You write the type between `<` and
`>`: `Core\Request::queryAs<Search>()`. A class needs the `#[Core\Json\Derive]` attribute, and each
public field reads the parameter with the same name. The address comes from the client, so every
string field must be `tainted string`.

Every value in a query string is text. A field of another type, such as `uint`, converts the text
the same way `as` does. Parameters that the type does not name are ignored.

The option `name` reads only the parameters inside one bracket name. For `price[min]=10`, you write
`{name: "price"}`, and the field `min` gets `10`.

If a field is missing or its value does not convert, `queryAs` throws a `ParseError`. Its `issues`
list has one entry for each wrong field.

**The examples below** read a search into a class, show the error for a wrong page number, and
filter a product list by a price range.
