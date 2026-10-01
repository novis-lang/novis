Joins a path to the folder the program was started from, and returns the full path.

A command-line tool uses it for a file name the user typed, such as `report.txt` in
`nvs run tool.nvs report.txt`. The user means a file in the folder they are working in, and
`fromCwd` gives the full path of that file. `.` and `..` are removed from the result. A path that
is already full is returned without the folder.

**Good to know:** `fromCwd` throws a `RuntimeError` while the program answers a web request. A
server's working folder is not the folder of your app. Write a path in your program as a string
literal instead, because a literal starts at the folder of the file that contains it.

**The examples below** turn a typed file name into a full path, show that `.` and `..` are
removed, and build export paths below the starting folder.
