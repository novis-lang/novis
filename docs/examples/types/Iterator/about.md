A cursor that walks a sequence once, front to back.

A class implements it with two methods: one steps to the next element and says whether there was
one, the other reads the element that step landed on. You fix the element type where you declare the
class, as in `Iterator<string>`. `foreach` calls both methods for you, and so does `Core\Arr::from`,
so a cursor of your own stands anywhere a sequence is expected.

Walking a cursor uses it up. There is no rewind and no second pass, so a second `foreach` over the
same one finds nothing left. For a collection somebody should be able to walk again, implement
`Iterable`: that hands out a fresh cursor every time.

**Good to know:** a cursor has no keys, so a `foreach` over one binds a single variable. Reading the
element before the first step, or after the sequence has ended, throws a `LogicError`.
