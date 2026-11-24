Checks a link that `signed` made. It checks that nobody changed the address, and that the link has
not expired. It returns nothing. When the check fails, it throws a `RuntimeError`, so the code after
the call runs only for a valid link.

You give it the same list of keys that `signed` used. A link signed with any key in the list is
accepted. To change to a new key, put the new key first and keep the old one for a while. Links
signed with the old key keep working until you remove it.

A changed parameter, a changed `_sig` value, a missing `_sig` and a wrong key all give the same
error message. An expired link gives a different message, so you can tell the user to ask for a new link.

**The examples below** check a link and a changed copy of it, and change to a new key. The last one
gives one reply to an expired download link and another reply to a changed one.
