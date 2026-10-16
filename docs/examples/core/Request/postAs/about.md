Reads a form that a browser sent with `POST` into a class or a shape that you write. You write the
type between `<` and `>`: `Core\Request::postAs<SignUp>()`. A class needs the
`#[Core\Json\Derive]` attribute, and each public field reads the form field with the same name. The
form comes from the client, so every string field must be `tainted string`.

Every value in a form is text. A field of another type, such as `uint`, converts the text the same
way `as` does. Form fields that the type does not name are ignored. `postAs` reads both kinds of
form: `application/x-www-form-urlencoded` and `multipart/form-data`.

The option `name` reads only the fields inside one bracket name. For `address[city]=Paris`, you
write `{name: "address"}`, and the field `city` gets `Paris`.

If a field is missing or its value does not convert, `postAs` throws a `ParseError`. Its `issues`
list has one entry for each wrong field.

**The examples below** read a sign-up form into a class, show the error for a wrong age, and read
the lines of an order form into a list.
