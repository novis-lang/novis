Returns the SID in its binary form. Active Directory stores a SID in this form, for example in
`objectSid`.

Use it to compare a SID with the bytes a directory returns, or to store a SID in a database
column of type `bytes`.

**Good to know:** the result has 8 bytes, then 4 bytes for each number after the authority. A SID
with 5 numbers, such as a user's SID, has 28 bytes.
