Every name in a program says what kind of value it holds, and says it once.

A local, a parameter, a result, a property, a constant, a loop counter and the name a `foreach`
hands out each round all write their kind down where they are introduced. From then on the kind is
settled: a name that started out holding a whole number holds one for the rest of its life, and
handing it text instead is refused before the program runs rather than at three in the morning.
Where the value on the right already makes the kind plain, `var` takes it from there and fixes it
just the same.

A name can be introduced without a value and filled in later. Reading one on a path where nothing
has given it a value is refused too, so there is no quiet empty value waiting to be tripped over.

**Good to know:** a name belongs to the whole function rather than to the block it stands in, so a
loop's counter can still be read after the loop. Introducing the same name twice is an error, not a
second name hiding the first.
