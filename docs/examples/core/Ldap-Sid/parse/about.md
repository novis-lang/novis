Reads SID text, such as `S-1-5-32-544`, and returns a `Core\Ldap\Sid`. A SID (security
identifier) is the number Windows uses for a user, a group or a computer.

Use it for a SID that you wrote yourself or that a user typed. Then you can compare it with the
SIDs a directory returns, or read its parts with `domain` and `rid`.

**Good to know:** the text starts with `S-1-`, then the authority, then 1 to 15 numbers. Each number
is at most `4294967295`. Text that is not a SID throws a `LogicError`, and its message says which part
is wrong.
