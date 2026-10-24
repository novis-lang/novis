Starts timing every call your program makes, until `Core\Debug::stopProfile`.

`Core\Debug::startProfile` writes the result to the file you name, in the speedscope format. You
can open that file in the speedscope viewer to see which calls took the most time.

The configuration must allow a profile. `[debug] mode` must include `"profile"`, and
`[capabilities.debug] profile` must include the folder of the file. Permission to write files is
not enough. Otherwise `startProfile` throws a `RuntimeError`. A second `startProfile` before the
stop throws a `LogicError`.

**Good to know:** timing a call adds a small cost to it. So code with many small calls looks
slower in the profile than it really is.

**The examples below** profile a report, show that only one profile runs at a time, and show the
two settings that a profile needs.
