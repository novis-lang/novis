Checks if the user must change the password at the next login. The result is `true` or `false`.

Active Directory has no bit in `userAccountControl` for this. It stores `0` in `pwdLastSet`
instead. `mustChangePassword` returns `true` when `pwdLastSet` is `0`. It returns `false` when
`pwdLastSet` has another value or the entry has no `pwdLastSet`.

You do not need to select `pwdLastSet` yourself. A search that selects `userAccountControl` also
reads `pwdLastSet`. `with` keeps this value, because it only changes the bits of
`userAccountControl`.
