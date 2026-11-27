An anonymous object is an object with exactly the fields you write. You do not declare a class for
it.

You write it as `{ x: 1, y: 2 }` and read or change a field with `$point->x`. Its type is the shape
`{ x: int, y: int }`, and a `type` alias gives that shape a short name. A variable with at least those
fields fits where the shape is needed. An object you write directly in that place may have only
the fields of the shape.
A field that may be missing is written `label?: string`, and `??` gives a value for the times it is
not there.

**Good to know:** an object is a handle and not a value. Two variables can name one object, and `==`
checks whether they do. It does not compare the fields, so two anonymous objects with equal values are
never equal.

**The examples below** build a point and change it, then pass shapes into a method and back out,
then return the result of a check as an object. Example 04 returns an object from an anonymous
function, and example 05 does not compile.
