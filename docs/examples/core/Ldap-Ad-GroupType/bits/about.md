Returns the whole number of `groupType`, exactly as Active Directory stores it. Each flag of the
group type is one bit of this number.

The security flag is the highest bit, `0x80000000`. Active Directory reads the number as signed, so
`bits` returns a negative number for a security group. A global security group is `-2147483646`, and
a global distribution group is `2`.

Use `bits` when you need the number itself, for example to write a changed group type back to the
directory. To check one flag, call the function for that flag, such as `security()`.
