Returns a copy of the account flags with some flags set or cleared, such as
`$flags->with({ disabled: true })`. Each option is one flag: `true` sets it, and `false` clears it. A
flag you do not name stays as it is.

The object you call `with` on does not change. Call `bits()` on the copy to get the new number of
`userAccountControl`, for example to write it back to the directory.

`lockedOut` and `passwordExpired` are not options. Active Directory computes these two flags, so a
program cannot set them. The copy keeps their values and the value of `mustChangePassword`.
