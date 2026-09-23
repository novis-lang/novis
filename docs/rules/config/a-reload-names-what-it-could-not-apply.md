The answer to a reload names, in one place: the directives applied and now in force; the **`Boot`
keys whose values changed and therefore did not take effect, each named individually**; and how many
compiled units were invalidated, so an operator knows a recompile wave is coming. A validation failure
reports the offending line and states that the running configuration is unchanged.

Naming the ignored `Boot` keys is the difference between a reload an operator can trust and one they
have to guess about: silently ignoring a changed listen address is how a deployment ends up believing
it applied a change it did not. The report and the carry are one operation — the published snapshot
still holds the *running* value of each named key, so the change exists nowhere but the report until
a restart. A changed `Boot` key is therefore absent from the applied list rather than present in
both.

A pending restart is loud. The reload that first sees a written value of a `Boot` key logs the key
with its running value and its written value, once for that written value and not once per reload.
`nvs ctl status` lists every pending key, one per line with both values, until the process restarts.
A file changed back to the running value clears the entry.

The unit count follows `rule:config/the-extension-set-is-in-every-unit-key`: a changed `env_hash`
invalidates every unit and an unchanged one invalidates none. What a reload cannot catch is an
extension removed while source still references it — those units fail when next resolved
(`rule:config/a-broken-edit-fails-the-requests-that-resolve-it`).
