There is a single `Isolate` type, and **an inbound HTTP request is the root isolate of a request
tree**. The server path and the `spawn script` path are then the same code: one arena setup, one
construction of the accessor classes' backing state, one config-overlay derivation, one teardown, one
place a limit is enforced.

That is worth more than it sounds. The cross-request state-bleed suite is simultaneously the
state-bleed suite for isolates, and a fix on either path cannot forget the other — which is the
property a second implementation would quietly give up, since two isolation mechanisms are two places
for the same bug to be fixed once.
