Writes a property of an object when the name of the property is in a string.

`set($object, $name, $value)` writes `$value` into the property `$name` of `$object`. The object
must be an instance of the described class. `set` follows the same rules as a normal write at the
same place in your code. A private property cannot be written from outside its class, and there is
no way to switch this check off. The value must have the type the property declares. If the class
watches its properties, `set` informs it the same way a normal write does.

A private property and a value of the wrong type throw a `RuntimeError`. A name that is not a
property of the class throws a `LogicError`. After an error, the object is unchanged. It replaces
PHP's `ReflectionProperty::setValue`.

**The examples below** write two properties by name, show the three errors, and fill a settings
object from configuration values.
