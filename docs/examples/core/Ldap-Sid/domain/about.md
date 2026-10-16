Returns the SID without its last number. For a user or a group, this is the SID of its domain.
For `S-1-5-21-1004336348-1177238915-682003330-1105` the result is
`S-1-5-21-1004336348-1177238915-682003330`.

Use it to check if two accounts are in the same domain, or to group SIDs by their domain.

**Good to know:** a SID with only one number after the authority, such as `S-1-1-0`, has no domain,
and the result is `null`.
