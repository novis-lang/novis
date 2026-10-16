An object is never copied when your program passes it from one place to another. A variable, a
property, an array element and a parameter all hold a reference to the object. Assigning it to a
second variable gives that variable the same object. Passing it to a function gives the function the
same object. A write through any one of them is seen through all of them.

`==` on two objects tests whether they are the same object. Two separate objects are never equal,
even when every property in them has the same value.

`clone` makes a new object of the same class and copies every property into it. The copy is one
level deep. An array property is copied, so the two objects have two arrays. An object property is
shared, so both objects point at the same inner object. No method runs during a clone.

**The examples below** show two variables naming one object, a function that changes the object it
is given, and a copy that is edited without changing the original.
