A class that implements `PropertyObserver` is told about every read and every write of its own
properties.

You write the two members `onPropertyGet` and `onPropertySet` once, and they are called for every
property the class declares. Each is handed the property's name and the value that has just settled:
the value the reader received, or the value that was stored. Both return nothing, so an observer
reports a change and never changes it. It replaces PHP's `__get` and `__set`, which Novis does not
recognize.

**Good to know:** the report always comes second. A property with a `set` block of its own runs that
block first, so the observer is told what was really stored and not what the caller passed. A class
that does not implement the interface pays nothing for one. And an observer that writes a property
of its own class is called again for that write, so its body has to be the thing that stops.
