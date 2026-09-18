What an open file is allowed to do, and what opening it does to whatever is already in the file. It
replaces the mode string PHP's `fopen` takes.

There are four. `Read` only reads, from a file that must already exist. `Write` only writes, and
empties the file the moment you open it, creating it if it is not there. `Append` only writes, always
at the end, keeps what is there, and creates the file if it is not there. `ReadWrite` does both,
creates the file if it is missing, and empties nothing.

`Write` is the one to be careful with. The file is empty from the moment the handle exists, whether
or not you ever write to it.

**Good to know:** there is no binary or text flag to choose. Novis text is bytes, so nothing changes
line endings for you on the way in or out.
