A short tour of the shapes almost every Novis program uses in its first ten lines.

A program is a file of statements that run from the top down. Every name you create says what kind of
value it holds, once, where you create it, and it holds that kind of value for the rest of its life.
A loop says the same thing about what it takes out of a list. When a value has to become something
else — the text a form sent, into a number you can add up — you ask for it with `as`, and a value
that cannot make
that trip stops the program with an error rather than quietly becoming zero. Everything the language
hands you is a member of a class, so there are no loose functions to look up. Nothing reaches outside
the program on its own: reading a file, calling another server or starting a program is refused until
whoever runs it has allowed that.

**The examples below** show these shapes at work one task at a time: adding up what a form sent, then
sorting a list with the one optional setting `sort` takes, then building a JSON answer to send back.
