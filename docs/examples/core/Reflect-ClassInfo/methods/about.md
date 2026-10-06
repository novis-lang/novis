Lists every method of a class.

`methods()` returns an array with one `Core\Reflect\MethodInfo` for each method of the described
class. The array is sorted by name. Each item gives the method's name with `name()`, whether it is
public with `isPublic()`, and its number of parameters with `parameterCount()`. The list contains
the methods the class inherits from its parent classes, and the constructor if the class has one.

Private methods are in the list too, with `isPublic()` returning `false`. The list only describes
the methods. To run one, use `call`, which checks the visibility.

**The examples below** show the list for one class, the methods a subclass inherits, and a help
text built from the public methods of a class.
