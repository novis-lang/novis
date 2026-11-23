//! `rule:config/three-changeability-classes`: the one list of keys `Core\Config::set` may change.
//!
//! `nvs.toml` gives every key the value a request starts with. [`SETTABLE`] names the keys a
//! request may then change for itself, one row per leaf key, and says how. A key it does not list
//! is refused before any other check, so a key added to the configuration is not settable until
//! somebody adds its row here. The checks of the value that follow — its unit, its ceiling, a pair
//! that must stay meaningful, the mode's ceiling — are [`Request::set`](crate::Request::set)'s.
//!
//! **A package sets only the `limits.*` rows**, and only lower
//! (`rule:security/package-authority-is-granted-one-line-at-a-time`); that is
//! [`Request::set_as_package`](crate::Request::set_as_package)'s.
//!
//! The list is closed in both directions: `crates/nvs-config/tests/request.rs` walks every key the
//! default file writes and checks `set` accepts exactly these, and
//! `crates/nvs-cli/tests/request_http_settings.rs` checks that setting each one changes what the
//! request does, so a row whose key nothing reads is found.
//!
//! Cost: one `&'static` slice of 37 rows, no allocation. A lookup is a linear scan, run once per
//! `set` and once per `set` or `restore` call site while compiling.

/// How a request may change a key on [`SETTABLE`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum How {
    /// Any valid value, wider or narrower, up to the key's ceiling: `[limits.hard]` (or the value
    /// the run started with) for a limit, `[mode] ceiling` for `mode.default`, and none for every
    /// other key.
    UpToCeiling,
    /// The value `false` and nothing else. A request may turn the switch off for itself, and no
    /// request turns it on; `restore` puts the file's value back.
    OffOnly,
}

impl How {
    /// The name `nvs meta` and the website print for this case.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            How::UpToCeiling => "up-to-ceiling",
            How::OffOnly => "off-only",
        }
    }
}

/// One key a request may change, and how.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settable {
    /// The dotted leaf key, `limits.memory`.
    pub key: &'static str,
    /// How a request may change it.
    pub how: How,
}

/// Every key a request may change. Order is presentation only: block by block, as the default
/// file writes them.
#[rustfmt::skip] // one row per line: the list is a table and reads as one.
pub const SETTABLE: &[Settable] = &[
    // `rule:config/a-program-may-read-and-flip-its-mode`: a request may flip its own mode, up to `[mode] ceiling`.
    Settable { key: "mode.default", how: How::UpToCeiling },
    // `rule:config/ceilings-are-their-own-directives`: each limit is the default a request starts
    // with, and `[limits.hard]` is the ceiling it may raise itself to. The limits not listed here
    // (the reserves, the depth, the decompression bounds, the high-water share and the disconnect
    // pair) protect the host or the failure report, so only the operator sets them.
    Settable { key: "limits.memory", how: How::UpToCeiling },
    Settable { key: "limits.cpu_time", how: How::UpToCeiling },
    Settable { key: "limits.wall_time", how: How::UpToCeiling },
    Settable { key: "limits.max_output", how: How::UpToCeiling },
    Settable { key: "limits.request_body", how: How::UpToCeiling },
    Settable { key: "limits.upload_total", how: How::UpToCeiling },
    Settable { key: "limits.max_multipart_parts", how: How::UpToCeiling },
    Settable { key: "limits.max_tasks", how: How::UpToCeiling },
    Settable { key: "limits.max_regex_steps", how: How::UpToCeiling },
    // `rule:config/a-mode-is-five-defaults`: the log's format and level are rows of the mode table.
    // The access line is not listed: it is written once the program has ended, from the file's
    // value (`nvs_server::trace::access_on`), so a request's own value would change nothing. Where
    // the log goes, its bounds, the handler and the client address are the operator's.
    Settable { key: "log.format", how: How::UpToCeiling },
    Settable { key: "log.level", how: How::UpToCeiling },
    // `rule:errors/a-use-of-deprecated-code-may-log-or-throw`: a test or one request turns it on for itself.
    Settable { key: "errors.deprecated", how: How::UpToCeiling },
    // `rule:concurrency/deferred-is-bounded-by-two-directives`: the deadline is a per-request
    // default a call may name its own value for. The cap sizes the host, so it is not listed.
    Settable { key: "deferred.deadline", how: How::UpToCeiling },
    // `rule:errors/debug-dump`: a request may turn its own inline dumps off and never on.
    Settable { key: "debug.inline", how: How::OffOnly },
    // `rule:testing/writing-a-debug-record-to-disk-is-its-own-gate`: a request may turn its own recording off and never on.
    Settable { key: "debug.store", how: How::OffOnly },
    // `rule:http-server/policy-headers-are-runtime-class-and-setheader-wins`: a request could write
    // each of these headers directly, so it may set the policy for its own response.
    Settable { key: "http.headers.content_type_options", how: How::UpToCeiling },
    Settable { key: "http.headers.frame_ancestors", how: How::UpToCeiling },
    Settable { key: "http.headers.referrer_policy", how: How::UpToCeiling },
    Settable { key: "http.headers.hsts", how: How::UpToCeiling },
    Settable { key: "http.headers.hsts_subdomains", how: How::UpToCeiling },
    Settable { key: "http.headers.content_security_policy", how: How::UpToCeiling },
    Settable { key: "http.headers.permissions_policy", how: How::UpToCeiling },
    // `rule:http-server/cors-is-closed-until-origins-are-named`: these shape the CORS headers of
    // this request's own response. `methods`, `headers` and `max_age` shape only a preflight,
    // which is answered before any program runs, so they are not listed.
    Settable { key: "http.cors.origins", how: How::UpToCeiling },
    Settable { key: "http.cors.expose", how: How::UpToCeiling },
    Settable { key: "http.cors.credentials", how: How::UpToCeiling },
    // `rule:http-server/cookies-are-secure-httponly-and-lax`: the attributes of the cookies this request sets.
    Settable { key: "http.cookies.secure", how: How::UpToCeiling },
    Settable { key: "http.cookies.http_only", how: How::UpToCeiling },
    Settable { key: "http.cookies.same_site", how: How::UpToCeiling },
    Settable { key: "http.cookies.path", how: How::UpToCeiling },
    // `rule:http-server/no-spelling-for-an-unbounded-wait`: each bounds one outbound call this request
    // makes. The pool's caps bound a core's memory, and TLS and the proxy are shared by every
    // request, so none of those is listed.
    Settable { key: "http.client.connect_timeout", how: How::UpToCeiling },
    Settable { key: "http.client.deadline", how: How::UpToCeiling },
    Settable { key: "http.client.max_redirects", how: How::UpToCeiling },
    Settable { key: "http.client.idle", how: How::UpToCeiling },
    Settable { key: "http.client.max_duration", how: How::UpToCeiling },
    // `rule:http-server/an-outbound-socket-is-bounded-by-idle-a-lifetime-and-a-message-cap`: each bounds one socket this request opens.
    Settable { key: "http.client.socket.max_message", how: How::UpToCeiling },
    Settable { key: "http.client.socket.send_timeout", how: How::UpToCeiling },
];

/// How a request may change the key `name` names, or `None` where it may not change it at all.
///
/// `name` is what a program writes: a dotted key, or a bare limit name such as `memory`, which
/// is the `[limits]` key of that name (`crate::request`'s module doc).
#[must_use]
pub fn how(name: &str) -> Option<How> {
    let key = crate::request::canonical(name);
    SETTABLE
        .iter()
        .find(|row| row.key == key)
        .map(|row| row.how)
}
