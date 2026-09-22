//! `Core\Server` — the class an application asks about the server it is running
//! under, replacing `$_SERVER`
//! (`rule:statements/no-host-populated-variables`).
//!
//! # What is here, and what is not
//!
//! One member: `isDraining()`, which is
//! `rule:http-server/the-server-block-is-boot-class`
//! 's last sentence — the same fact `[server] health_path` answers a proxy
//! with, given to an application for an endpoint of its own. The rest of
//! [docs/spec/01-core-library.md](/docs/spec/01-core-library.md) § 15's
//! `Core\Server` — the request's own environment and `traceId()` — is a known
//! gap of this module and not of the spec: both read per-request state that a
//! served request's context does not carry yet.
//!
//! # Why the bit is not read from `nvs-server`
//!
//! It could not be: this crate does not depend on that one, and should not —
//! a `Core` member that needed the HTTP server linked in to answer a question
//! about the process would put `hyper` in the graph of every CLI program.
//! [`nvs_runtime::drain`] is the crate both rest on, and it owns the atomic,
//! its ordering, and why a real server takes the process's bit. `nvs_server`'s
//! accept loop is the only writer; everything here is a reader.
//!
//! **A program that is not being served reads `false`**, and that is the
//! answer rather than an error: a CLI program, a scheduled script and a test
//! all run in a process no accept loop is draining, and asking whether a
//! shutdown has begun is a question they may ask and be told no.

use nvs_runtime::Value;

use crate::registry::{CoreClass, CoreMethod, CoreTy, MethodDoc};

/// `Core\Server`'s registry rows — § 15's `isDraining`, and so far nothing else.
pub(crate) const CLASS: CoreClass = CoreClass {
    name: r"Core\Server",
    doc: None,
    methods: &[CoreMethod {
        name: "isDraining",
        names: &[],
        params: &[],
        defaults: &[],
        return_ty: CoreTy::Bool,
        symbol: "nvs_core_server_is_draining",
        doc: Some(&IS_DRAINING_DOC),
    }],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\Server::isDraining`'s reference card — `rule:core-api/reference-card`.
const IS_DRAINING_DOC: MethodDoc = MethodDoc {
    short: "Reports whether this server has begun a graceful shutdown — the same fact `[server] \
            health_path` answers a proxy with, for an application endpoint of its own.",
    params: &[],
    ret: "`true` once the server has stopped accepting connections, `false` while it is still \
          accepting and in any process that is not serving.",
    errors: &[],
};

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::symbols`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_server_is_draining" => (nvs_core_server_is_draining as *const ()).cast(),
        _ => return None,
    })
}

nvs_runtime::nvs_helper! {
    /// `Core\Server::isDraining(): bool` — `rule:http-server/the-server-block-is-boot-class`'s drain, read by the
    /// application rather than by the built-in probe.
    ///
    /// One relaxed load of the process's bit, and deliberately not a snapshot
    /// taken at the start of the request: a shutdown that begins mid-request is
    /// exactly what this exists to report, so an answer cached for the length of
    /// a request would be stale in the only window anyone asks in.
    fn nvs_core_server_is_draining(_ctx, _args: [0]) {
        Ok(Value::bool(nvs_runtime::drain::is_draining()))
    }
}

#[cfg(test)]
mod tests {
    use nvs_runtime::drain::Drain;

    /// The member reads the *process's* bit and not one of its own.
    ///
    /// Asserted through [`Drain::process`] rather than by calling the helper
    /// with a `Ctx`: what could go wrong here is the member reading a detached
    /// handle, and that is a question about which bit, not about the ABI.
    #[test]
    fn the_member_reads_the_processs_own_bit() {
        assert_eq!(
            nvs_runtime::drain::is_draining(),
            Drain::process().is_draining(),
            "`isDraining` answered from a bit no server writes"
        );
    }
}
