A class can watch every read and every write of its own properties by implementing the interface
`PropertyObserver`.

You write two methods, `onPropertyGet` and `onPropertySet`. Novis calls them for every property the
class declares: public or private, with hooks or without, written in the constructor or later, and
declared by a subclass too. Each call gives you the property name and the value after it has settled,
so `onPropertySet` sees what was really stored.

An observer reports, and it does not decide. Both methods return nothing, so they cannot change the
value a read gives back. A method that throws an error stops that read or that write.

**Good to know:** writing a property inside one of these two methods calls the method again. Test the
name first, or the program runs until the call stack is too deep.

**The examples below** show the two methods on a small class, how far one observer reaches, and a
record that knows when it needs saving.
