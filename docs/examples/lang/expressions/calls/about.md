A call runs a method and gives back whatever that method returns.

You write the class or the object, the method's name, and the arguments in brackets. Results chain,
so the object one call gives back is what the next call is written on. Arguments go in the order the
method declares them, or by name in any order — which is also how you leave out a parameter in the
middle that already has a default. A method whose last parameter takes the rest accepts as many
arguments as you pass, and a list you already hold is poured in whole. A parameter marked `inout` is
written back into the variable you passed, and you write `inout` at the call as well, so the change
is visible where it happens.

**Good to know:** a value that holds a function — in a variable, an array element or a property — is
called through whatever holds it, and its arguments are checked while the program runs rather than
before it starts.

**The examples below** take these in turn: calling and chaining, naming the arguments you pass, and
pricing an order with the handler the customer chose.
