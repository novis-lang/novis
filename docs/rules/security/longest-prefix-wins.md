Lookup reuses the resolution rule a developer already learned. A bare prefix grants **that namespace's
own declarations only**; a subtree is spelled with an explicit wildcard and grants every depth below
it. The **longest matching key wins**, an exact key beats a subtree key at equal length, and two keys
that would match identically is an error naming both when the table is read.

**The subtree form is spelled out because it is the dangerous one.** A subtree grant reaches code that
does not exist yet: granting a vendor's whole subtree means a module introduced by an upgrade eight
months from now holds the database. That is the opposite of the posture everywhere else, so it is
never the default reading of a bare prefix — the narrow thing is what a reader gets, and the broad
thing costs two extra characters and is visible in review as its own token.

**Declined:**
- **A bare prefix meaning the subtree.** It saves one form and makes the dangerous reading the
  default, so a grant would silently extend to modules that did not exist when it was written.

**On disk.** `nvs_config::grants::Grants::lookup` is the lookup, and the tie is `E0656`.
`nvs_types::capability::reject_ungranted_call` asks it for the namespace enclosing each `Core` call
site.
