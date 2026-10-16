Returns the name of a class constant.

`name()` returns the name of the constant that this `Core\Reflect\ConstantInfo` describes. The
name is written exactly as the class declares it, in upper case like `MAX_ITEMS`. It has no class
name in front of it and no `::`. A text that is equal to the name only when case is ignored, such
as `max_items`, is not the name.

Private and protected constants have a name too, and `name()` returns it. To read the value, pass
the name to `Core\Reflect\ClassInfo::constant`.

**The examples below** show how to print every constant name of a class, how to find a name that
was typed with the wrong case, and how to check the keys of a configuration against the constant
names of a class.
