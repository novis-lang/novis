A call runs a method and returns the method's result.

You write the class or the object, the name of the method, and the arguments in brackets. You can
call a method on the result of a call directly. Arguments go in the order of the parameters. You can
also write them by name, in any order. A name also lets you leave out a parameter in the middle that
has a default. A variadic parameter, written `...$rest`, takes any number of arguments. `...$list`
at the call passes each value of an array as an argument. An `inout` parameter writes its new value
back to the variable you passed. You write `inout` at the call as well.

**Good to know:** you call a `callable` through the variable, the array element or the property
that contains it. Its arguments are checked when the program runs.

**The examples below** take these in turn: calling and chaining, naming the arguments you pass, and
pricing an order with the handler the customer chose. The last one shows each way to write a call in
one program.
