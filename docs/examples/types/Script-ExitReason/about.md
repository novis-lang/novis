Which ending ran your cleanup. A script can register work to do on the way out, and whatever that
work is, its first question is usually how the script ended — you tidy up differently after a clean
run than after a crash.

There are four answers. `Normal` means the last line ran and the script simply finished. `ExitCall`
means `exit` ended it. `UncaughtThrow` means something was thrown and nothing caught it, and the
report hands you the error itself. `Finish` means the script stopped itself early on purpose, which
is what a request handler does once it has answered.

**Good to know:** there is no case for a crash or a cancellation, and that is not an oversight — when
either of those ends a script, no cleanup runs at all, so there is nothing to be told. Every reason
you can ever be handed is one of the four.
