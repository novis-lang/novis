Novis reads the owner and the permissions of a file or folder to check who can write to it. An
owner that is a member of `Administrators` passes this check.

`nvs init` offers to restrict the folder it writes into. The `icacls` commands that this example
prints are for folders you choose yourself, such as a separate config, cache or logs folder, and
for the folder that contains one of them.

On every drive except the system drive, Windows gives the group `Authenticated Users` the right to
change each new folder. Every account on the computer is in that group, so such a folder fails the
check. Two `icacls` commands repair it. `/inheritance:d` stops the folder from inheriting
permissions from the drive. `/remove:g` then removes the group, also from the folders inside.

Use the SID of a group in `icacls`. Windows translates group names, and a SID is the same on every
Windows.

**Good to know:** `/remove:g` removes every right of the group, also the right to read. A grant of
`RX` to the group `Users` gives reading back, and the folder still passes the check.

**The example below** prints the commands for one folder and one account.
