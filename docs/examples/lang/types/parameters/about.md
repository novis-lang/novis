A method lists the values it needs as parameters, and the call says which value goes to which.

Each parameter has a type and a name. A parameter with a default can be left out by the caller. The
default can be a value, a class constant, or an enum case. The last parameter can be
variadic: it collects the remaining arguments into a list. At the call, you can write an argument
with its parameter's name in front of it, as in `book(day: "Friday")`. Then you can skip the
defaults you do not need. Arguments without a name come first.

A parameter written `inout` changes the caller's own variable when the method returns. The call
writes `inout` in front of that argument too, so you can see the change at the call.

**Good to know:** parameter names are part of what a method promises, so a caller may rely on them.

**The examples below** show defaults and a variadic parameter, then named arguments with an enum
case as a default, then an `inout` stock count.
