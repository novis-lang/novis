Lists every attribute written on a class and on its properties, methods and parameters.

`attributes()` returns one `Core\Reflect\AttributeInfo` for each `#[...]` in the class
declaration, in the order they are written. Each one says where it is written and what its
fields are. A class with no attributes gives an empty list.

The list only has the attributes written in this class's own declaration. A subclass does not
list the attributes of its parent class. To read those, describe the parent class too.

**Good to know:** if you know the attribute's type when you write the code, `Core\Attributes::get`
is simpler and is checked when the program compiles.

**The examples below** show where each attribute of a class is written, a subclass that lists
only its own attribute, and a form checked against rules written as attributes.
