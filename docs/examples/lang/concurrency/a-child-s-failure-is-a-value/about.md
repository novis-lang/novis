A script you start with `spawn script` runs on its own. When it fails, the failure comes back to you as
a value.

`await` returns a result. `ok` is false when the script ended with an error it did not catch. `error` has
the class name and the message of that error. `output` still has everything the script printed before it
failed. Your program keeps running.

A mistake in your own program is different. A file name that points to nothing throws an error in your
program. So does a file that does not compile, or a misspelled `output:` setting. Those errors happen at
the line that starts the script, and you catch them with `try`.

**The examples below** show a failed script read as a result, your own mistake told apart from the
script's failure, and a group of jobs where one fails and the other two still finish.
