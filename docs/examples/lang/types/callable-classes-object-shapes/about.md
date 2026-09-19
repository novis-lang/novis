The types that name a doer or a thing rather than a plain value: a function you can pass around, a
class, any object at all, and an object described by the fields it carries.

`callable` holds a function written with `fn`, and nothing else — a name kept in a piece of text is
never callable here. A class, interface or enum name is a type wherever a type is written, and
`object` is the one type every instance fits, with `is` and `as` getting the real class back out of
it. Where a program has to pick a class while it is running, it carries a class reference instead of
the class's name as text: turn the name into one once, and create, call and test through the result
afterwards. A shape describes an object by the fields it has rather than the class it came from, and
an object literal builds one on the spot; a value carrying more fields than the shape asks for still
fits it.

**Good to know:** a call through a `callable` answers a value of unknown kind, so say what it is
where you use it — `$f(4) as int`.
