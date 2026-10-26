The file with the token that opens the debug viewer. There is no default.

`nvs debug` needs a token when `[debug] listen` is an address that is not loopback. Without this
setting, `nvs debug` creates a random token in `debug/` in the data folder. It prints a link that
contains the token. Open that link once, and the browser keeps the token in a cookie.

Set this when you want to choose the token yourself. The file contains the token on one line. The
token has at least 16 characters. Each character is a letter, a digit, `-`, `_`, `.` or `~`. If the
token is not valid, `nvs debug` stops with an error. If the file does not exist, `nvs debug` prints a
warning and uses the token it created.

`nvs debug` reads this setting when it starts. A program cannot change it.
