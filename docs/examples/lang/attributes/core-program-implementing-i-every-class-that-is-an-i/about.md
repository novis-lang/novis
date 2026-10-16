`Core\Program::implementing<I>()` gives you one object for every class in your program that
implements the interface `I`. Novis works that list out while it compiles your program, so nothing is
registered while it runs and no list of classes is written anywhere. A class joins the list by
implementing the interface.

The array is sorted by the full class name. The order is the same on every machine and never depends
on the file system. An `abstract` class is not in the list, because `new` cannot create one. Every
class in the list needs a constructor that takes no arguments, and what a class needs arrives through
the interface's own methods instead. An interface that no class implements gives an empty array.

The objects are created where the call stands, once each time it runs, like any other `new`. Asking
twice gives you two separate sets of objects.

The examples show a list of checks, which classes end up in the list, and one table written in every
format a program knows.
