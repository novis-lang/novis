Registers a function that runs when a resource limit stops your program.

A limit stops the program when it uses too much memory or too much CPU time. A `catch` block cannot
catch this error, so this function is the only code that still runs. It gets an array whose `limit`
key names the limit, for example `memory` or `cpu_time`. It runs once. After it returns, the program
ends.

Novis keeps back a small amount of memory and time for this function, so it can still print or log
a message. Keep it short. If it throws an error or uses up that small amount too, it is stopped.

**Good to know:** only one function is registered. A second call replaces the first one.

**The examples below** show the name of the limit, a function that ignores the array, and a short
message for the person who started a big job.
