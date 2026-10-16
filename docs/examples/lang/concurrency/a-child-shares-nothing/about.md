A script you start with `spawn script` runs on its own. It shares nothing with the program that started
it.

Every class static in the started script begins at its declared value, whatever your program wrote in its
own copy. A write inside the script stays there. Your own values are the same after the script has
finished as they were before it began. Two scripts started one after another share nothing either.

Printing works the same way. With `output: "capture"` the text the script prints comes back to you in the
`output` field of the result, and nothing reaches the screen. With `output: "inherit"` the script prints
to your screen, and `output` is then empty.

**The examples below** show a script that cannot change your numbers, the two ways to handle what a
script prints, and a health check made of three scripts that cannot disturb each other.
