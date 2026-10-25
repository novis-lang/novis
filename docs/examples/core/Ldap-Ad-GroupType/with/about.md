Returns a copy of a group type with some flags set or cleared, such as
`$type->with({ security: false })`. Each option is one flag: `true` sets it, and `false` clears it. A
flag you do not name stays as it is.

The object you call `with` on does not change. Call `bits()` on the copy to get the new number of
`groupType`, for example to write it back to the directory.

`with` does not check that the flags make sense together. A group has exactly one scope, so when you
change the scope, clear the old one and set the new one in the same call.
