When `nvs init` or the compile cache cannot use a folder, the message names a path and says what is wrong with it.

The path in the message is the one to change. It can be the folder one level above the folder you
named. Two messages are about permissions that are too open. In the first, a group of ordinary
accounts can write to the path. Remove that right. On Windows the message prints the SID of the
group. In the second, another ordinary account owns the path. Change the owner to the account that
runs `nvs`.

`Access is denied` and `Permission denied` are different. The path passed the check, and your own
account cannot write to it. `it already exists` means that `nvs init` found a file at that path and
did not change it.

**Good to know:** a message that starts with `warning:` does not stop the program. Novis does not
use the cache folder until you fix the path, so it compiles the program again at every start.

**The example below** reads five messages and prints the path to change and what to do.
