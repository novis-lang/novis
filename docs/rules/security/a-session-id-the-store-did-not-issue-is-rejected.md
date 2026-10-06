`Core\Session::start()` accepts a presented id only if its own store issued it and it is still
live. An unknown, expired or attacker-minted id is discarded and a fresh id is issued in its place.
There is no toggle, no ini, no option, on the security priority: an off switch for fixation
resistance is a security default traded for nothing. Session fixation resistance is therefore a
property of the language, not of a deployment's configuration.

It follows that the session store must be able to answer "did I issue this id". A store that holds
nothing, such as a signed cookie carrying the record itself, cannot answer it and so cannot be the
session store (`rule:core-api/session-roster`). The acceptance test presents a fabricated id and
asserts that a fresh one comes back.
