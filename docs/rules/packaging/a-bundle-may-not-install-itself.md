`nvs service install` refuses when the running binary is a single-file bundle, with a diagnostic
(`E0634`) naming the reason.

A bundle is a single trust domain because the person who downloads and runs it is the only principal
involved (`rule:programs/bundle-trust-domain`). Installing a service creates a **second principal** — a
privileged account executing that payload at every boot, with no operator having read what it
contains. That is the operator-versus-app-author boundary the bundle declined to cross, arrived at from
the other side, and the answer has to be the same one.

The narrower rule — allow it for a non-`serve` payload — is rejected as a conditional a reader must
carry in their head to serve a case nobody has asked for. Reopening this means arguing the trust-domain
point directly.
