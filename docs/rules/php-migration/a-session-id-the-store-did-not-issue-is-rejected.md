`Core\Session::start()` accepts a presented id only if its own store issued it and it is still
live. An unknown, expired or attacker-minted id is discarded and a fresh id is issued in its place.
There is no toggle, no ini, no option: PHP 8.6 flips `session.use_strict_mode` to `1` by default,
and Novis has no off position, on the first priority — an off switch for fixation resistance is a
security default traded for nothing. Session fixation resistance is therefore a property of the
language, not of a deployment's ini hygiene.

It follows that the session store must be able to answer "did I issue this id" — the same demand
PHP 8.6 makes of session handlers by deprecating those without `create_sid`/`validateId`. A store
that holds nothing, such as a signed cookie carrying the record itself, cannot answer it and so
cannot be the session store (`rule:core-api/session-roster`). The acceptance test presents a
fabricated id and asserts that a fresh one comes back.
