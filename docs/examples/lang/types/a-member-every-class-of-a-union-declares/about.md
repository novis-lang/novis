A variable of a union of classes, such as `Box|Circle`, is an object of one of those classes. You
can read a property or call a method of it when every class has that member with the same type. A
method needs the same parameters and return type. The member must also be visible where you use it.

Use `?->` on a union that may be `null`, such as `?Box|Circle`. A write compiles when no class makes
the property `readonly` and every class has the same `set` visibility. `isinit` does not compile on
a property of a union.

If one class does not have the member, the program does not compile. The same is true when the
types differ, or when the member is not visible in one class. The error is `E0881`, and its message
names the class. An interface is not allowed in such a union.

**The examples below** read a property of two shipping classes and call a method of two payment
classes. The last one is a compile error.
