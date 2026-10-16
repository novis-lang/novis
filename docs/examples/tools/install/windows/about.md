On Windows, Novis reads the owner and the permissions of a file or folder to check who can write to it.

On every drive except the system drive, Windows gives the group `Authenticated Users` the right to
change each new folder. Every account on the computer is in that group, so such a folder fails the
check. Two `icacls` commands repair it. `/inheritance:d` stops the folder from inheriting
permissions from the drive. `/remove:g` then removes the group. The folders inside get the same
permissions.

Use the SID of a group in `icacls`, not its name. Windows translates group names, and a SID is the
same on every Windows. When the check fails, the error message prints the SID to remove.

**Good to know:** `/remove:g` removes every right of the group, also the right to read. A grant of
`RX` to the group `Users` gives reading and running back to all accounts, and the folder still
passes the check.

**The example below** prints the commands for one folder and one account.
