`Core\Session::start()` accepts a presented id only if its own store issued it and it is still
live. An unknown, expired or attacker-minted id is discarded and a fresh, empty session is opened in
its place. Its new id is drawn when something is first written to it
(`rule:core-classes/session-is-started-explicitly`), so the presented id is never used.
There is no toggle, no ini, no option, on the security priority: an off switch for fixation
resistance is a security default traded for nothing. Session fixation resistance is therefore a
property of the language, not of a deployment's configuration.

A store row belongs to one application: the innermost `[[app]]` block that matched the request, the
one `crate::cache::application` reads. An id that another application's row holds is absent, so a
session one mount issued is never opened under another mount that shares the store. Both stores key
the row by the application and the id (`rule:http-server/no-session-block-means-the-data-folder-store`).
`nvs_stdlib::session`'s test `a_session_one_application_wrote_is_absent_in_another_on_every_store`
and `per_mount_config`'s `two_mounts_do_not_open_each_others_sessions` prove it.

It follows that the session store must be able to answer "did I issue this id". A store that holds
nothing, such as a signed cookie carrying the record itself, cannot answer it and so cannot be the
session store (`rule:core-api/session-roster`). `nvs_stdlib::session`'s test
`start_opens_a_held_record_and_a_fresh_one_for_an_id_the_store_does_not_hold` presents a forged id
and gets a fresh session that does not carry it.
