Stops the profile that `Core\Debug::startProfile` started, and closes its file.

Calls that run after `Core\Debug::stopProfile` are not timed. So you can profile one slow step of
a program and leave out the rest. After the stop, you can start a new profile with another file,
for example one file for each step.

When no profile is running, `stopProfile` does nothing. So you can call it in a `finally` block,
and the profile stops even when a step throws an error.

**The examples below** profile one step of an upload, stop a profile in a `finally` block, and
write one profile for each step of a job.
