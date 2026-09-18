Keeps the scratch directories a program asked for, instead of deleting them the moment it ends.

A program that needs somewhere to put working files gets its own directory, and that directory is
deleted when the program finishes — whether it ended normally, stopped early or died of an error.
Switch this on and nothing is deleted; each kept directory is written to the log instead, so you can
go and look at what a failing run left behind. Switch it off again afterwards, because nothing else
ever cleans those directories up.

**Good to know:** only an operator can set it. A program cannot exempt its own files from the
cleanup, because a program that could would be one an attacker could talk into filling the disk.
