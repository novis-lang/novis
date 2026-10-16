Checks a link that `sign` signed. It checks that nobody changed the address, and that the link has
not expired. It returns nothing. When the check fails, it throws a `RuntimeError`, so the code after
the call runs only for a valid link.

You give it the same list of keys that `sign` used. A link signed with any key in the list is
accepted. To change to a new key, put the new key first and keep the old one for a while. Links
signed with the old key keep working until you remove it.

A changed parameter, a changed token, a missing `_sig` and a wrong key all give the same error
message. An expired link gives a different message, so you can tell the user to ask for a new link.

**The examples below** check a link and a changed copy of it, change to a new key, and answer an
expired download link differently from a changed one.
