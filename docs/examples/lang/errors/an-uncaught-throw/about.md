A throw that no `catch` handles ends your program.

The program stops at the line that threw. Everything it printed before that line is still printed.
The exit status is 1. The error output gets a report with the error class, the message, and one line
for each function the throw passed through, innermost first.

There is no global handler that catches everything for you and lets the program carry on. Catch the
error where you can do something about it, or let it end the program.

Two methods let you watch this ending without changing it. `Core\Fatal::onUncaughtThrow` gives you
the error itself. `Core\Script::onExit` runs at every ending, normal or not. Both run before the
program exits, and the exit status is still 1.

**The examples below** show the plain ending first, then each of these two methods.
