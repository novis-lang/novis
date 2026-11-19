Returns the name of the class that the described class extends.

`parent()` returns the full name of the parent class, written the same way `name()` writes a
name. It returns `null` for a class that does not extend another class. It also returns `null`
for an interface, because an interface has no parent class.

`attributes()` lists only the attributes written in one class. To read the attributes of every
base class too, start at a class and call `Core\Reflect::forClass($info->parent())` in a loop. The
loop stops when `parent()` returns `null`. `forClass` also returns `null` for a `Core` class, so
the loop stops there too.

**Good to know:** `parent()` returns a name, not a description. Call `Core\Reflect::forClass` with
the name when you need the description of the parent class.

**The examples below** print the parent of a class, walk from a class up to its root class, and
read the attributes of every base class.
