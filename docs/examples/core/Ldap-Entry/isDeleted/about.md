Checks whether an entry is a deleted entry. It returns `true` or `false`.

Active Directory does not remove a deleted account at once. It keeps the entry for some time, with
most of its attributes removed. The SID and the GUID stay. A normal search does not find these
entries. A search with `showDeleted: true` finds them too, and `isDeleted()` tells them apart from
the active ones.

The account that searches needs the right to read deleted entries. An administrator has it. Without
that right, the server does not send the `isDeleted` attribute, so `isDeleted()` returns `false`.

**Good to know:** a deleted entry has a new DN under `CN=Deleted Objects`. To find a deleted
account again, search by a value that stays, such as its SID.
