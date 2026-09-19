A Novis program is one file you run, and the lines written in it are the program.

There is no build step and no starting function to write. You save the file, you run it, and the
statements in it happen from the top downwards, in the order you wrote them. `echo` writes values
out, one after another, and puts nothing between them; `print` does the same for a single value.
Classes are different: one can be written anywhere in the file, above or below the lines that use
it, because declaring a class is not a step that happens at a moment.

**Good to know:** nothing is added to what you write out — not even a line break. A program that
should end its line says so, by writing the newline itself.
