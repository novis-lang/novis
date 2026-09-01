//! The `[db.<name>]` paths that name a file, resolved and trust-checked at boot.
//!
//! One field today — [`Database::tls_ca_file`](crate::tree::Database::tls_ca_file), the PEM bundle
//! ADR 0067 § 3's TLS leg verifies a server's certificate against. It is here rather than in
//! [`mod@crate::secret`] because it is not a secret: nothing about a CA bundle is confidential, and
//! § 7's whole shape — one value, materialized into a sibling directive, kept out of the merged
//! table — is the wrong one for a file a connection re-reads by path. What it *shares* with § 7 is
//! the trust boundary, which is why both run at resolve time and neither at use time.
//!
//! **A CA bundle is a trust-boundary file.** Whoever can rewrite it decides which server this
//! deployment's queries and credentials go to, which is the same authority ADR 0103 § 6 refuses to
//! leave on the configuration files themselves. So it goes through
//! [`Files::trust`] exactly as a `password_file` does, and an
//! unreadable or group-writable bundle is `E0605`/`E0607` at boot. It is deliberately *not* read
//! here: the anchors are parsed by `nvs_host::tls`, which owns the one client configuration every
//! session in this process shares, and a second parse in this crate would be a second answer to
//! "whose certificates do you believe".
//!
//! **The path is rewritten in place**, absolute, exactly as [`mod@crate::app`] canonicalizes a
//! block's key and for its second reason: § 9's `nvs config dump` then prints the file the
//! handshake will actually open rather than a fragment whose meaning depends on which file in the
//! tree wrote it. A connection opened from a working directory that is not the configuration's —
//! every request, since ADR 0103 § 5 resolves against the *file* — would otherwise read a different
//! bundle or none.

use std::collections::BTreeMap;
use std::path::Path;

use nvs_diagnostics::Diagnostic;

use crate::resolve::{Files, Origin};
use crate::tree::Config;

/// Makes every `[db.<name>] tls_ca_file` absolute and proves it is inside the trust boundary.
///
/// Runs over the merged tree for [`mod@crate::secret`]'s reason: which bundle is in force is a
/// question only the merge has answered, and trust-checking a path a later file replaced would
/// refuse a boot over a file the deployment does not use.
///
/// # Errors
///
/// `E0607` for a bundle outside ADR 0103 § 6's trust boundary and `E0605` for one that cannot be
/// read at all — [`crate::resolve::untrusted`]'s split, so an operator told "cannot read" goes
/// looking for a typo and one told the other goes looking at a mode.
pub fn canonicalize(
    config: &mut Config,
    origins: &BTreeMap<String, Origin>,
    files: &dyn Files,
) -> Result<(), Diagnostic> {
    for (name, db) in &mut config.db {
        let Some(written) = db.tls_ca_file.as_deref() else {
            continue;
        };
        // § 5: relative to the file it was written in. A key with no origin cannot have been
        // written anywhere, so there is nothing but the path itself to resolve against.
        let base = origins
            .get(&format!("db.{name}.tls_ca_file"))
            .and_then(|origin| origin.path.parent())
            .unwrap_or(Path::new("."));
        let path = crate::resolve::absolute(base, Path::new(written));
        let trusted = files.trust(&path).map_err(|why| {
            crate::resolve::untrusted(
                &path,
                &why,
                "a `[db]` block's `tls_ca_file` names it, and whoever can write it chooses which \
                 server the connection may be talking to",
            )
        })?;
        db.tls_ca_file = Some(trusted.to_string_lossy().into_owned());
    }
    Ok(())
}
