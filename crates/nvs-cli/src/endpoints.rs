//! The two URLs the `nvs` tool sends to, read from the environment
//! (`rule:config/telemetry-and-update-endpoints-are-configuration`).
//!
//! `NVS_UPDATE_URL` replaces [`RELEASES`], the GitHub Releases API's latest release of the public
//! repository, which is all `nvs update-check` needs and is served by nobody but GitHub.
//! `NVS_TELEMETRY_URL` is the telemetry upload's only URL: with it unset nothing is uploaded,
//! because no collector exists and a default would send data to a host nobody runs. Neither is an
//! `nvs.toml` key: they configure the tool, not a deployment. An empty value counts as unset.
//!
//! Each is read when it is asked for and never kept, so a test points one child process at its
//! own listener. Cost: one environment read per command that sends.

/// The variable that replaces [`RELEASES`].
pub(crate) const UPDATE_VARIABLE: &str = "NVS_UPDATE_URL";

/// The variable that names where telemetry is uploaded.
pub(crate) const TELEMETRY_VARIABLE: &str = "NVS_TELEMETRY_URL";

/// The latest release of `https://github.com/novis-lang/novis`, as the Releases API serves it.
pub(crate) const RELEASES: &str = "https://api.github.com/repos/novis-lang/novis/releases/latest";

/// Where `nvs update-check` reads the latest release: `NVS_UPDATE_URL`, or [`RELEASES`].
pub(crate) fn update_url() -> String {
    set(UPDATE_VARIABLE).unwrap_or_else(|| RELEASES.to_owned())
}

/// Where telemetry is uploaded: `NVS_TELEMETRY_URL`, or `None`, which uploads nothing.
pub(crate) fn telemetry_url() -> Option<String> {
    set(TELEMETRY_VARIABLE)
}

/// `name`'s value with surrounding whitespace removed, or `None` where it is unset, empty or not
/// Unicode.
fn set(name: &str) -> Option<String> {
    let value = std::env::var(name).ok()?;
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_owned())
}
