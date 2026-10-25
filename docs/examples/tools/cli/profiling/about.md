`nvs run --profile` and `nvs test --profile` measure how long each function call takes. You do not change your program for this.

When the run ends, Novis writes a profile file in the current folder. `--profile callgrind` writes `callgrind.out.<pid>`, with a total time for each function and a list of who called it. You open it in KCachegrind or Webgrind. `--profile speedscope` writes `profile.<pid>.speedscope.json`, with every call on a timeline. You open it in speedscope or in VS Code. `--profile-out <file>` writes the profile to another file.

**Good to know:** timing a call adds a small cost to it. So code with many small calls looks slower in the profile than it really is.

**The examples below** profile one part of a program with `Core\Debug::startProfile`, which writes the same speedscope file. They read the file back and print which functions ran, how often, and in what order.
