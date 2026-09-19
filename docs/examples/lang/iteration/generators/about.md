A generator is a method that gives out its values one at a time, as a loop asks for them.

Write `yield` in the body and declare `Iterator<T>` as the return type. Calling the method runs none
of the body. It returns a cursor, and each step of a `foreach` loop runs the body as far as the next
`yield`. When the loop ends, the body stops where it is. This is what makes a generator cheap: it
holds one value at a time, and it may describe a sequence that goes on forever.

**Good to know:** a generator is walked once. A second `foreach` over the same cursor sees nothing,
and calling the method again gives you a new cursor that starts at the beginning.

**The examples below** show a small generator first, then a sequence with no end, then two
generators reading one report together.
