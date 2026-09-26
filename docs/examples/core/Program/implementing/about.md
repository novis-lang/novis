Returns a new object of every class that implements an interface.

You write the interface between `<` and `>`, for example `Core\Program::implementing<Module>()`. The
result is an array with one new object per class, sorted by class name. Abstract classes are not in
the array. Each class needs a constructor without arguments. A framework uses this to find its
plugins, commands or checks, so nobody has to keep a list of them up to date.

**Good to know:** Novis finds the classes when it compiles your program, so nothing is searched while
the program runs. Every call creates new objects.

**The examples below** run every health check of a service, pick an export format by its name, and
print a help page that lists every command.
