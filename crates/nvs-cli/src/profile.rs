//! `--profile` and `--profile-out` on `nvs run` and `nvs test`:
//! `rule:observability/speedscope-timeline-export`'s two exports of `rule:testing/debug-surface`'s
//! profiler, wrapped around a whole run with no source change.
//!
//! The host installs a [`nvs_runtime::Profiler`] on the run's context, so every context the run
//! makes has a track of its own in the one file ([`nvs_runtime::Ctx::set_run_profile`]). The file
//! goes to the working folder under the name each viewer opens by default, unless `--profile-out`
//! names another.

use std::path::{Path, PathBuf};
use std::sync::Arc;

/// The two exports `--profile` writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub(crate) enum Format {
    /// Self and inclusive time per function, for KCachegrind and Webgrind.
    Callgrind,
    /// Every call on one timeline, for speedscope.
    Speedscope,
}

impl Format {
    fn runtime(self) -> nvs_runtime::ProfileFormat {
        match self {
            Self::Callgrind => nvs_runtime::ProfileFormat::Callgrind,
            Self::Speedscope => nvs_runtime::ProfileFormat::Speedscope,
        }
    }

    /// The file name in the working folder: `callgrind.out.<pid>`, the name KCachegrind looks
    /// for, or `profile.<pid>.speedscope.json`.
    fn default_path(self) -> PathBuf {
        let pid = std::process::id();
        PathBuf::from(match self {
            Self::Callgrind => format!("callgrind.out.{pid}"),
            Self::Speedscope => format!("profile.{pid}.speedscope.json"),
        })
    }
}

/// A run's profile while it runs.
pub(crate) struct Running {
    profiler: Arc<nvs_runtime::Profiler>,
}

/// Creates the profile file and installs the profiler on `ctx`, with the timing probe on.
/// `name` is what the profile is of, such as the program's file.
///
/// # Errors
///
/// The line to print when the file cannot be created.
pub(crate) fn install(
    ctx: &mut nvs_runtime::Ctx,
    format: Format,
    out: Option<&Path>,
    name: &str,
) -> Result<Running, String> {
    let path = out.map_or_else(|| format.default_path(), Path::to_path_buf);
    let profiler = nvs_runtime::Profiler::create(&path, format.runtime(), name).map_err(|err| {
        format!(
            "error: cannot create the profile `{}`: {err}",
            path.display()
        )
    })?;
    ctx.set_run_profile(Some(Arc::clone(&profiler)));
    ctx.set_profile_sink(Some(profiler.track(name)));
    ctx.set_debug_flags(ctx.debug_flags() | nvs_runtime::DebugFlags::PROFILE);
    Ok(Running { profiler })
}

impl Running {
    /// Closes `ctx`'s track and writes the rest of the file, then prints where it went on stderr.
    /// Returns `false`, having printed why, when a write failed.
    pub(crate) fn finish(self, ctx: &mut nvs_runtime::Ctx) -> bool {
        drop(ctx.set_profile_sink(None));
        ctx.set_run_profile(None);
        let path = self.profiler.path().display().to_string();
        match self.profiler.finish() {
            Ok(()) => {
                eprintln!("profile: {path}");
                true
            }
            Err(err) => {
                eprintln!("error: could not write every call to the profile `{path}`: {err}");
                false
            }
        }
    }
}
