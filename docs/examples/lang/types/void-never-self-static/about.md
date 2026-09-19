Four return types say things an ordinary type cannot.

`void` says the call produces no value. The method is called for what it does, and what comes back
cannot be printed, given a name or passed on. `never` says the method does not come back at all: it
throws, or it ends the program.

`self` and `static` both name a class, and the difference is which one. `self` is the class the
method was written in. `static` is the class the call actually named, so a base class can hand back
an instance of whichever subclass was asked for, and the caller is given that subclass rather than
the base.

The examples show a method called only for its effect, a chain of calls that each hand the object
back, and a base class that builds whichever kind was asked for.
