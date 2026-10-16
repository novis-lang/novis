Lets a class of your own say how two of its values are ordered, so everything that sorts can sort them.

A class that implements `Comparable` has one member, `compareTo`. It takes another value of the same
class and answers a number: negative when this one comes first, zero when neither comes first,
positive when the other one does. From then on `<`, `>`, `<=`, `>=` and `<=>` work on those values,
and every member that puts things in order reads that same answer — sorting a list, picking the
smallest or the largest, keeping a queue in priority order. You write the ordering once, and nothing
else in your program needs its own.

**Good to know:** a class that says nothing about its order cannot be ordered at all. The comparison
is refused while your program is built, and the message names `Comparable` as the fix. Equality does
not change: two values are `==` only when they are the same value, so asking whether two values have
the same contents is `compareTo` answering zero.
