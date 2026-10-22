`Core\Session::start()` accepts a presented id only if its own store issued it and it is still
live. An unknown, expired or attacker-minted id is discarded and a fresh, empty session is opened in
its place. Its new id is drawn when something is first written to it
(`rule:core-classes/session-is-started-explicitly`), so the presented id is never used.
There is no toggle, no ini, no option, on the security priority: an off switch for fixation
resistance is a security default traded for nothing. Session fixation resistance is therefore a
property of the language, not of a deployment's configuration.

It follows that the session store must be able to answer "did I issue this id". A store that holds
nothing, such as a signed cookie carrying the record itself, cannot answer it and so cannot be the
session store (`rule:core-api/session-roster`). `nvs_stdlib::session`'s test
`start_opens_a_held_record_and_a_fresh_one_for_an_id_the_store_does_not_hold` presents a forged id
and gets a fresh session that does not carry it.
