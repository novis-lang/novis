//! `nvs telemetry` — `rule:tooling/telemetry-is-two-opt-ins`,
//! `rule:tooling/telemetry-counters-are-a-closed-set`,
//! `rule:tooling/telemetry-show-prints-the-payload`, `rule:tooling/a-serving-process-never-uploads`
//! and `rule:tooling/a-failed-upload-never-fails-the-command`.
//!
//! **Two plain files under the data folder.** `telemetry/consent` is `on` or `off`, and a folder
//! without it is off. `telemetry/counters` is one `key value` line each: the lifetime upload
//! totals, the current week's day bits and counters, and the last finished week that is not sent
//! yet. A counter file somebody edited is read through the same closed set, so a line that names
//! anything else is dropped and never reaches a payload. A week or a time outside [`YEARS`] and
//! [`TIMES`] is read as absent, and every count stops at `u64::MAX`, so no value in the file can
//! make the arithmetic overflow.
//!
//! **A counter name is only ever built from [`crate::COMMAND_NAMES`].** [`Counter`] keeps a
//! `&'static str` out of that table, or [`UNKNOWN`], so no byte of the command line can become a
//! name. [`start`] counts the invocation once the command is parsed, and [`unparsed`] counts a
//! command line that parsed to no command.
//!
//! **The panic hook counts and returns.** [`install_panic_hook`] runs before
//! `nvs_runtime::floor::install_panic_hook`, so the floor calls it only for a panic that no served
//! request claimed. It counts `internal-error.<subcommand>` once per process and then calls the
//! hook it replaced, so the panic prints exactly as it did.
//!
//! **A non-empty `DO_NOT_TRACK` or `CI` turns it all off for the process.** Nothing is recorded,
//! and `status` names the variable. The stored consent is left as it is.
//!
//! **Only a carrier uploads.** [`start`] is told whether the command may carry an upload, from
//! `main`'s exhaustive `carries_upload`, and [`report`] and `nvs telemetry upload` send nothing in
//! a process it said no for. An upload sends the `pending` week once its point has passed: the
//! Monday after the week plus a random offset under [`WINDOW`], drawn the first time a carrier
//! sees the week, and never inside the week of the last accepted upload. It is one `POST` of
//! [`Counters::payload`]'s bytes under [`BUDGET`], with no retry, and the reply decides the
//! week: `204` records it, `400`, `413` and `415` drop it, and everything else keeps it.
//!
//! Cost: with telemetry off, one environment read and one missing-file read per invocation. With
//! it on, one read and one rewrite of a file of a few KiB, and the folder's ownership check.
//! Two invocations that end at the same moment may lose one count, because the second rewrite
//! wins. An upload is one HTTPS request of a few hundred bytes, at most once a week, and at most
//! [`BUDGET`] of the carrying command's time. Nothing here runs on a request: `nvs serve` counts
//! once, before it binds, and never uploads.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant, SystemTime};

use rand::RngExt;

use crate::endpoints;

/// The fallback subcommand of every counter: a command line that names no command in the table.
pub(crate) const UNKNOWN: &str = "unknown";

/// The variables that turn telemetry off for one process, in the order `status` checks them.
const OPT_OUTS: [&str; 2] = ["DO_NOT_TRACK", "CI"];

/// The version every payload reports.
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The consent file's name inside the telemetry folder.
const CONSENT: &str = "consent";

/// The counter file's name inside the telemetry folder.
const COUNTERS: &str = "counters";

/// The counter file's first line.
const HEADER: &str =
    "# Usage counters of `nvs telemetry`. Run `nvs telemetry show` to see what an upload sends.";

/// How long an upload may take, connecting included.
const BUDGET: Duration = Duration::from_secs(3);

/// The seconds after a week ends that its upload point is drawn from, so carriers that run at
/// the same minute everywhere do not all send at once.
const WINDOW: i64 = 86_400;

/// Seconds in a day.
const DAY: i64 = 86_400;

/// The years a week in the counter file may name. A file somebody edited may name any number,
/// and a week outside these years is read as absent, so no date arithmetic on it can overflow.
const YEARS: std::ops::RangeInclusive<i64> = 1970..=9999;

/// The times in seconds `last` and `due` may hold: 1970-01-01 up to the end of 9999. A value
/// outside them is read as absent, for the same reason as [`YEARS`].
const TIMES: std::ops::RangeInclusive<i64> = 0..=253_402_300_799;

/// Whether this process's command may carry an upload, which [`start`] sets once.
static CARRIER: AtomicBool = AtomicBool::new(false);

/// Record anonymous usage counters, or stop recording them.
///
/// Telemetry is off until you turn it on. When it is on, each `nvs` command
/// adds 1 to a counter for that command, in a file in the data folder. The
/// file contains no paths, arguments or other data from your programs.
/// If `DO_NOT_TRACK` or `CI` is set, nothing is recorded.
#[derive(clap::Args)]
pub(crate) struct Args {
    #[command(subcommand)]
    verb: Verb,
}

/// `nvs telemetry`'s own subcommands.
#[derive(clap::Subcommand)]
enum Verb {
    /// Start recording counters.
    On,
    /// Stop recording counters. The counters already recorded are kept.
    Off,
    /// Print whether telemetry is on, where the counters are and the upload totals.
    Status,
    /// Delete the setting and every counter. Telemetry is then off.
    Reset,
    /// Print the exact JSON document the next upload sends.
    Show,
    /// Send the last finished week now, if it is due. `nvs update-check` also does this.
    Upload,
}

impl Args {
    /// Whether this command may carry an upload: only `nvs telemetry upload`.
    pub(crate) fn uploads(&self) -> bool {
        matches!(self.verb, Verb::Upload)
    }

    /// Runs the verb clap parsed against this process's data folder.
    pub(crate) fn run(self) -> ExitCode {
        let env = Env::read();
        let out = &mut std::io::stdout();
        if matches!(self.verb, Verb::On) {
            return match nvs_config::data::prepare() {
                Ok(folder) => on(&folder.telemetry(), &env, out),
                Err(unusable) => {
                    eprintln!("{}", unusable.warning());
                    eprintln!("error: telemetry needs the data folder, so it stays off");
                    ExitCode::FAILURE
                }
            };
        }
        let Some(folder) = nvs_config::data::current() else {
            let _ = writeln!(out, "telemetry: off");
            let _ = writeln!(out, "Novis has no data folder, so nothing is recorded.");
            return ExitCode::SUCCESS;
        };
        let dir = folder.telemetry();
        match self.verb {
            Verb::On => unreachable!("handled above"),
            Verb::Off => off(&dir, out),
            Verb::Status => status(&dir, &env, now(), out),
            Verb::Reset => reset(&dir, out),
            Verb::Show => show(&dir, today(), out),
            Verb::Upload => upload(&env, out),
        }
    }
}

/// What the environment says about telemetry for this process.
struct Env {
    /// The first variable of [`OPT_OUTS`] that is set and not empty.
    opt_out: Option<&'static str>,
    /// [`endpoints::telemetry_url`].
    upload: Option<String>,
}

impl Env {
    /// This process's environment.
    fn read() -> Self {
        Self {
            opt_out: opt_out(|name| std::env::var_os(name)),
            upload: endpoints::telemetry_url(),
        }
    }
}

/// The first of [`OPT_OUTS`] that `var` gives a non-empty value.
fn opt_out(var: impl Fn(&str) -> Option<OsString>) -> Option<&'static str> {
    OPT_OUTS
        .into_iter()
        .find(|name| var(name).is_some_and(|value| !value.is_empty()))
}

/// Whether the consent file in `dir` says `on`.
fn consented(dir: &Path) -> bool {
    std::fs::read_to_string(dir.join(CONSENT)).is_ok_and(|text| text.trim() == "on")
}

/// Whether telemetry records in this process: consent is `on` in the data folder and no variable
/// of [`OPT_OUTS`] is set. `nvs update-check` prints it.
pub(crate) fn is_on() -> bool {
    opt_out(|name| std::env::var_os(name)).is_none()
        && nvs_config::data::current().is_some_and(|folder| consented(&folder.telemetry()))
}

/// `nvs telemetry on`: writes the consent into `dir`, which it creates.
fn on(dir: &Path, env: &Env, out: &mut impl Write) -> ExitCode {
    if let Err(error) = write_consent(dir, "on") {
        eprintln!(
            "error: could not write `{}`: {error}",
            dir.join(CONSENT).display()
        );
        return ExitCode::FAILURE;
    }
    let _ = writeln!(out, "telemetry: on");
    let _ = writeln!(
        out,
        "Each `nvs` command adds 1 to a counter in {}.",
        dir.join(COUNTERS).display()
    );
    let _ = writeln!(out, "{}", upload_line(env.upload.as_deref()));
    if let Some(variable) = env.opt_out {
        let _ = writeln!(
            out,
            "`{variable}` is set, so nothing is recorded while it is set."
        );
    }
    ExitCode::SUCCESS
}

/// `nvs telemetry off`: writes `off` into a consent file that exists. Without one telemetry is
/// already off, and nothing is created.
fn off(dir: &Path, out: &mut impl Write) -> ExitCode {
    if dir.join(CONSENT).exists()
        && let Err(error) = write_consent(dir, "off")
    {
        eprintln!(
            "error: could not write `{}`: {error}",
            dir.join(CONSENT).display()
        );
        return ExitCode::FAILURE;
    }
    let _ = writeln!(out, "telemetry: off");
    if dir.join(COUNTERS).exists() {
        let _ = writeln!(
            out,
            "The counters already recorded are still in {}. Run `nvs telemetry reset` to delete them.",
            dir.join(COUNTERS).display()
        );
    }
    ExitCode::SUCCESS
}

/// `nvs telemetry status` at `now`, in seconds since 1970.
fn status(dir: &Path, env: &Env, now: i64, out: &mut impl Write) -> ExitCode {
    let stored = consented(dir);
    let on = stored && env.opt_out.is_none();
    let _ = writeln!(out, "telemetry: {}", if on { "on" } else { "off" });
    if let Some(variable) = env.opt_out {
        let _ = writeln!(
            out,
            "`{variable}` is set, so nothing is recorded or uploaded while it is set."
        );
        if stored {
            let _ = writeln!(out, "Without it, telemetry is on.");
        }
    }
    let _ = writeln!(out, "counters: {}", dir.join(COUNTERS).display());
    let _ = writeln!(out, "{}", upload_line(env.upload.as_deref()));
    let mut counters = Counters::read(dir);
    counters.roll(Week::of(now.div_euclid(DAY)));
    for line in counters.history(now).lines() {
        let _ = writeln!(out, "{line}");
    }
    ExitCode::SUCCESS
}

/// `nvs telemetry upload`: the upload this process carries, or why there is none. Always
/// [`ExitCode::SUCCESS`].
fn upload(env: &Env, out: &mut impl Write) -> ExitCode {
    let line = if let Some(variable) = env.opt_out {
        format!("telemetry: off, because `{variable}` is set. Nothing is uploaded.")
    } else if env.upload.is_none() {
        upload_line(None)
    } else {
        match Recorder::active() {
            None => "telemetry: off, so nothing is uploaded.".to_owned(),
            Some(recorder) => carry_from(&recorder.dir).map_or_else(
                || "This command does not upload.".to_owned(),
                |outcome| outcome.line(),
            ),
        }
    };
    let _ = writeln!(out, "{line}");
    ExitCode::SUCCESS
}

/// `nvs telemetry reset`: deletes `dir` and everything in it.
fn reset(dir: &Path, out: &mut impl Write) -> ExitCode {
    match std::fs::remove_dir_all(dir) {
        Ok(()) => {
            let _ = writeln!(out, "deleted {}", dir.display());
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let _ = writeln!(out, "nothing to delete: {} does not exist", dir.display());
        }
        Err(error) => {
            eprintln!("error: could not delete `{}`: {error}", dir.display());
            return ExitCode::FAILURE;
        }
    }
    let _ = writeln!(out, "telemetry: off");
    ExitCode::SUCCESS
}

/// `nvs telemetry show` on `day`: the payload alone on `out`, and on standard error a note when
/// the week it describes is not finished or there is nothing to send.
fn show(dir: &Path, day: i64, out: &mut impl Write) -> ExitCode {
    let mut counters = Counters::read(dir);
    counters.roll(Week::of(day));
    let (tally, finished) = match (&counters.pending, &counters.current) {
        (Some(pending), _) => (pending, true),
        (None, Some(current)) if !current.counts.is_empty() => (current, false),
        _ => {
            eprintln!("Nothing is recorded, so there is nothing to send.");
            return ExitCode::SUCCESS;
        }
    };
    let _ = writeln!(out, "{}", counters.payload(tally));
    if !finished {
        eprintln!(
            "Week {} is not finished. An upload sends a week only after it ends.",
            tally.week.label()
        );
    }
    ExitCode::SUCCESS
}

/// The line that says where an upload goes.
fn upload_line(upload: Option<&str>) -> String {
    upload.map_or_else(
        || {
            format!(
                "Nothing is uploaded, because `{}` is not set.",
                endpoints::TELEMETRY_VARIABLE
            )
        },
        |url| format!("upload URL: {url}"),
    )
}

/// Creates `dir` privately and writes `value` into its consent file.
fn write_consent(dir: &Path, value: &str) -> std::io::Result<()> {
    nvs_config::data::create_private_root(dir)?;
    replace(&dir.join(CONSENT), &format!("{value}\n"))
}

/// Writes `text` to `path` through a file beside it and a rename, so a reader never sees half
/// of it.
fn replace(path: &Path, text: &str) -> std::io::Result<()> {
    let mut beside = path.as_os_str().to_owned();
    beside.push(format!(".tmp-{}", std::process::id()));
    let beside = PathBuf::from(beside);
    std::fs::write(&beside, text)
        .and_then(|()| std::fs::rename(&beside, path))
        .inspect_err(|_| {
            let _ = std::fs::remove_file(&beside);
        })
}

/// Now, as seconds since 1970-01-01 in UTC.
fn now() -> i64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |since| i64::try_from(since.as_secs()).unwrap_or(0))
}

/// Today, as days since 1970-01-01 in UTC.
fn today() -> i64 {
    now().div_euclid(DAY)
}

/// `seconds` since 1970 as `2026-10-26T05:12:00Z`.
pub(crate) fn stamp(seconds: i64) -> String {
    let (year, month, day) = civil(seconds.div_euclid(DAY));
    let time = seconds.rem_euclid(DAY);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        time / 3_600,
        time / 60 % 60,
        time % 60
    )
}

/// An ISO 8601 week: weeks start on Monday, and week 1 is the one with the year's first Thursday.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Week {
    /// The ISO year, which is the calendar year of the week's Thursday.
    year: i64,
    /// 1 to 53.
    number: i64,
}

impl Week {
    /// The week `day` falls in.
    fn of(day: i64) -> Self {
        let thursday = day - weekday(day) + 3;
        let (year, _, _) = civil(thursday);
        Self {
            year,
            number: (thursday - days_from_civil(year, 1, 1)) / 7 + 1,
        }
    }

    /// `2026-W43`.
    fn label(self) -> String {
        format!("{}-W{:02}", self.year, self.number)
    }

    /// The day number of the Monday after this week, which is when it ends.
    fn end(self) -> i64 {
        // January 4th is always in week 1.
        let fourth = days_from_civil(self.year, 1, 4);
        fourth - weekday(fourth) + self.number * 7
    }

    /// The week [`Week::label`] wrote, with its year in [`YEARS`].
    fn parse(text: &str) -> Option<Self> {
        let (year, number) = text.split_once("-W")?;
        let week = Self {
            year: year.parse().ok()?,
            number: number.parse().ok()?,
        };
        (YEARS.contains(&week.year) && (1..=53).contains(&week.number)).then_some(week)
    }
}

/// `day`'s weekday, Monday `0` to Sunday `6`. 1970-01-01 was a Thursday.
fn weekday(day: i64) -> i64 {
    (day + 3).rem_euclid(7)
}

/// The calendar year, month and day of `day`.
pub(crate) fn civil(day: i64) -> (i64, i64, i64) {
    // The proleptic Gregorian calendar in 400-year eras, counted from 0000-03-01 so a leap day
    // is the last day of its year.
    let z = day + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    (
        yoe + era * 400 + i64::from(mp >= 10),
        month,
        doy - (153 * mp + 2) / 5 + 1,
    )
}

/// The day number of `year-month-day`.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let yoe = year - era * 400;
    let mp = if month > 2 { month - 3 } else { month + 9 };
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Which half of the counter set a counter is in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Kind {
    /// `invocation.<subcommand>`: one command ran.
    Invocation,
    /// `internal-error.<subcommand>`: that command ended in a panic of `nvs` itself.
    InternalError,
}

impl Kind {
    /// The name's first half.
    fn label(self) -> &'static str {
        match self {
            Self::Invocation => "invocation",
            Self::InternalError => "internal-error",
        }
    }
}

/// One counter of the closed set.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Counter {
    kind: Kind,
    /// An entry of [`crate::COMMAND_NAMES`], or [`UNKNOWN`].
    command: &'static str,
}

impl Counter {
    /// `kind` for `command`, which is mapped onto the table: a name the table does not have is
    /// [`UNKNOWN`].
    fn new(kind: Kind, command: &str) -> Self {
        Self {
            kind,
            command: crate::COMMAND_NAMES
                .iter()
                .find(|known| **known == command)
                .copied()
                .unwrap_or(UNKNOWN),
        }
    }

    /// The counter `name` spells, or `None` where it is not one of the closed set.
    fn parse(name: &str) -> Option<Self> {
        let (kind, command) = name.split_once('.')?;
        let kind = [Kind::Invocation, Kind::InternalError]
            .into_iter()
            .find(|known| known.label() == kind)?;
        let counter = Self::new(kind, command);
        (counter.command == command).then_some(counter)
    }

    /// `invocation.run`.
    fn name(self) -> String {
        format!("{}.{}", self.kind.label(), self.command)
    }
}

/// One week of counters.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Tally {
    week: Week,
    /// Bit `n` is set when `nvs` ran on weekday `n`, Monday `0`. The file writes it as seven
    /// digits, Monday first.
    days: u8,
    /// Every counter above zero.
    counts: BTreeMap<Counter, u64>,
}

impl Tally {
    /// An empty `week`.
    fn new(week: Week) -> Self {
        Self {
            week,
            days: 0,
            counts: BTreeMap::new(),
        }
    }
}

/// Everything the counter file has.
#[derive(Debug, Default, PartialEq, Eq)]
struct Counters {
    /// How many uploads the collector accepted, over the folder's life.
    uploads: u64,
    /// How many bytes those uploads sent.
    bytes: u64,
    /// When the collector last accepted an upload, in seconds since 1970.
    last: Option<i64>,
    /// When the pending week may be sent, in seconds since 1970. Drawn the first time a carrier
    /// sees that week, and `None` while nothing is pending.
    due: Option<i64>,
    /// The week counting now.
    current: Option<Tally>,
    /// The last finished week, not sent yet.
    pending: Option<Tally>,
}

impl Counters {
    /// The counter file in `dir`. A missing or unreadable file is empty, and a line that is not
    /// part of the format is left out.
    fn read(dir: &Path) -> Self {
        std::fs::read_to_string(dir.join(COUNTERS))
            .map(|text| Self::parse(&text))
            .unwrap_or_default()
    }

    /// The counter file's text.
    fn parse(text: &str) -> Self {
        let mut counters = Self::default();
        // Whether the `days` and counter lines below belong to the pending week; `None` before
        // either week line.
        let mut pending = None;
        for line in text.lines() {
            let line = line.trim();
            if line.starts_with('#') {
                continue;
            }
            let Some((key, value)) = line.split_once(' ') else {
                continue;
            };
            let value = value.trim();
            match key {
                "uploads" => counters.uploads = value.parse().unwrap_or(0),
                "bytes" => counters.bytes = value.parse().unwrap_or(0),
                "last" => counters.last = value.parse().ok().filter(|time| TIMES.contains(time)),
                "due" => counters.due = value.parse().ok().filter(|time| TIMES.contains(time)),
                "week" => {
                    counters.current = Week::parse(value).map(Tally::new);
                    pending = Some(false);
                }
                "pending" => {
                    counters.pending = Week::parse(value).map(Tally::new);
                    pending = Some(true);
                }
                _ => {
                    let tally = match pending {
                        Some(false) => counters.current.as_mut(),
                        Some(true) => counters.pending.as_mut(),
                        None => None,
                    };
                    let Some(tally) = tally else { continue };
                    if key == "days" {
                        tally.days = value
                            .chars()
                            .take(7)
                            .enumerate()
                            .filter(|(_, digit)| *digit == '1')
                            .fold(0, |days, (day, _)| days | 1 << day);
                    } else if let (Some(counter), Ok(count)) =
                        (Counter::parse(key), value.parse::<u64>())
                        && count > 0
                    {
                        tally.counts.insert(counter, count);
                    }
                }
            }
        }
        counters.pending = counters.pending.filter(|tally| !tally.counts.is_empty());
        if counters.pending.is_none() {
            counters.due = None;
        }
        counters
    }

    /// The counter file's text, which [`Counters::parse`] reads back.
    fn text(&self) -> String {
        use std::fmt::Write as _;

        let mut text = format!("{HEADER}\nuploads {}\nbytes {}\n", self.uploads, self.bytes);
        for (key, value) in [("last", self.last), ("due", self.due)] {
            if let Some(value) = value {
                let _ = writeln!(text, "{key} {value}");
            }
        }
        for (key, tally) in [("week", &self.current), ("pending", &self.pending)] {
            let Some(tally) = tally else { continue };
            let _ = writeln!(text, "{key} {}", tally.week.label());
            let days: String = (0..7)
                .map(|day| if tally.days & 1 << day == 0 { '0' } else { '1' })
                .collect();
            let _ = writeln!(text, "days {days}");
            for (counter, count) in &tally.counts {
                let _ = writeln!(text, "{} {count}", counter.name());
            }
        }
        text
    }

    /// Makes `week` the current one. A current week that ended becomes the pending one when it
    /// counted anything, and a week after `week`, which only a clock set back can leave, is
    /// dropped and never sent. A new pending week has no upload point yet.
    fn roll(&mut self, week: Week) {
        match self.current.take() {
            Some(tally) if tally.week == week => self.current = Some(tally),
            Some(tally) if tally.week < week && !tally.counts.is_empty() => {
                self.pending = Some(tally);
                self.due = None;
            }
            _ => {}
        }
        if self
            .pending
            .as_ref()
            .is_some_and(|tally| tally.week >= week)
        {
            self.pending = None;
        }
        if self.pending.is_none() {
            self.due = None;
        }
        self.current.get_or_insert_with(|| Tally::new(week));
    }

    /// The earliest moment the next upload may be sent, after [`Counters::roll`] to `now`'s week:
    /// the pending week's point, or the end of the current week while nothing is pending, and
    /// never inside the week of the last accepted upload.
    fn earliest(&self, now: i64) -> i64 {
        let week = self
            .pending
            .as_ref()
            .map_or_else(|| Week::of(now.div_euclid(DAY)), |tally| tally.week);
        let due = self.due.unwrap_or(week.end() * DAY);
        let after_last = self
            .last
            .map_or(0, |last| Week::of(last.div_euclid(DAY)).end() * DAY);
        due.max(after_last)
    }

    /// What `status` and `nvs update-check` print about uploads at `now`.
    fn history(&self, now: i64) -> History {
        History {
            last: self.last,
            next: self.earliest(now),
            uploads: self.uploads,
            bytes: self.bytes,
        }
    }

    /// Sends the pending week to `url` at `now` when it is due, within `within`. `jitter` is the
    /// offset into [`WINDOW`] a week with no upload point yet gets. What the reply decides is
    /// written into `self`, and nothing is retried.
    fn send(&mut self, url: &str, now: i64, jitter: i64, within: Duration) -> Outcome {
        let Some(pending) = self.pending.clone() else {
            return Outcome::Nothing;
        };
        let week = pending.week.label();
        self.due.get_or_insert(pending.week.end() * DAY + jitter);
        let at = self.earliest(now);
        if now < at {
            return Outcome::NotDue { week, at };
        }
        let payload = self.payload(&pending);
        let bytes = u64::try_from(payload.len()).unwrap_or(u64::MAX);
        let headers = vec![("User-Agent".to_owned(), format!("nvs/{VERSION}"))];
        let started = Instant::now();
        let reply = nvs_stdlib::tool_post(
            "nvs telemetry",
            url,
            headers,
            "application/json",
            payload.into_bytes(),
            within,
        );
        match reply {
            Ok(reply) if reply.status == 204 => {
                self.uploads = self.uploads.saturating_add(1);
                self.bytes = self.bytes.saturating_add(bytes);
                self.last = Some(now);
                self.pending = None;
                self.due = None;
                Outcome::Sent { week, bytes }
            }
            Ok(reply) if matches!(reply.status, 400 | 413 | 415) => {
                self.pending = None;
                self.due = None;
                Outcome::Dropped {
                    status: reply.status,
                    field: named(&reply.body),
                }
            }
            Ok(reply) => Outcome::Kept(format!("the collector returned {}", reply.status)),
            Err(_) if started.elapsed() >= within => {
                Outcome::Kept(format!("timeout after {within:?}"))
            }
            Err(error) => Outcome::Kept(error),
        }
    }

    /// Adds 1 to `counter` on `day`, and sets that day's bit.
    fn count(&mut self, counter: Counter, day: i64) {
        self.roll(Week::of(day));
        let tally = self
            .current
            .get_or_insert_with(|| Tally::new(Week::of(day)));
        tally.days |= 1 << weekday(day);
        let count = tally.counts.entry(counter).or_insert(0);
        *count = count.saturating_add(1);
    }

    /// The JSON document an upload of `tally` sends, with its keys in the rule's order.
    fn payload(&self, tally: &Tally) -> String {
        let counts = tally
            .counts
            .iter()
            .map(|(counter, count)| format!("\"{}\":{count}", counter.name()))
            .collect::<Vec<_>>()
            .join(",");
        format!(
            r#"{{"schema":1,"version":{},"os":"{}","arch":"{}","week":"{}","first":{},"days_active":{},"counters":{{{counts}}}}}"#,
            serde_json::Value::from(VERSION),
            os(std::env::consts::OS),
            arch(std::env::consts::ARCH),
            tally.week.label(),
            self.uploads == 0,
            tally.days.count_ones(),
        )
    }
}

/// `os` mapped onto the payload's closed list.
fn os(os: &str) -> &'static str {
    match os {
        "linux" => "linux",
        "macos" => "macos",
        "windows" => "windows",
        _ => "other",
    }
}

/// `arch` mapped onto the payload's closed list.
fn arch(arch: &str) -> &'static str {
    match arch {
        "x86_64" => "x86_64",
        "aarch64" => "aarch64",
        _ => "other",
    }
}

/// The field a refused report names: the reply's first line, printable ASCII only, at most 64
/// characters, so the collector cannot write control characters to the terminal.
fn named(body: &[u8]) -> String {
    let text = String::from_utf8_lossy(body);
    let first: String = text
        .lines()
        .next()
        .unwrap_or_default()
        .chars()
        .filter(|c| c.is_ascii_graphic() || *c == ' ')
        .take(64)
        .collect();
    first.trim().to_owned()
}

/// What one upload attempt did.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Outcome {
    /// No finished week is waiting.
    Nothing,
    /// `week` may be sent from `at`, in seconds since 1970.
    NotDue { week: String, at: i64 },
    /// The collector accepted `week`'s `bytes`.
    Sent { week: String, bytes: u64 },
    /// The week is kept for the next carrier, for the reason given.
    Kept(String),
    /// The collector refused the week with `status`, naming `field`, and it is deleted.
    Dropped { status: i64, field: String },
}

impl Outcome {
    /// `--json`'s name for this outcome.
    pub(crate) fn result(&self) -> &'static str {
        match self {
            Self::Nothing => "nothing",
            Self::NotDue { .. } => "not-due",
            Self::Sent { .. } => "sent",
            Self::Kept(_) => "kept",
            Self::Dropped { .. } => "dropped",
        }
    }

    /// The one line the carrying command prints.
    pub(crate) fn line(&self) -> String {
        match self {
            Self::Nothing => "telemetry: no finished week is waiting to be sent".to_owned(),
            Self::NotDue { week, at } => {
                format!("telemetry: week {week} is sent from {}", stamp(*at))
            }
            Self::Sent { week, bytes } => format!("telemetry: sent week {week}, {bytes} bytes"),
            Self::Kept(why) => format!("telemetry: could not send ({why}); kept for next time"),
            Self::Dropped { status, field } if field.is_empty() => {
                format!("telemetry: refused by the collector ({status}); dropped")
            }
            Self::Dropped { status, field } => {
                format!("telemetry: refused by the collector ({status}: {field}); dropped")
            }
        }
    }
}

/// The upload history `status` and `nvs update-check` print.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct History {
    /// When the collector last accepted an upload, in seconds since 1970.
    pub(crate) last: Option<i64>,
    /// The earliest moment the next upload may be sent, in seconds since 1970.
    pub(crate) next: i64,
    /// How many uploads the collector accepted.
    pub(crate) uploads: u64,
    /// How many bytes those uploads sent.
    pub(crate) bytes: u64,
}

impl History {
    /// The three lines, in the order they are printed.
    pub(crate) fn lines(&self) -> [String; 3] {
        [
            format!(
                "last upload: {}",
                self.last.map_or_else(|| "never".to_owned(), stamp)
            ),
            format!("next upload: from {}", stamp(self.next)),
            format!(
                "uploads accepted: {}, bytes sent: {}",
                self.uploads, self.bytes
            ),
        ]
    }
}

/// What `nvs update-check` adds while telemetry is on.
pub(crate) struct Report {
    /// The upload this process carried, or `None` where it carried none.
    pub(crate) outcome: Option<Outcome>,
    /// The history after it.
    pub(crate) history: History,
}

/// The upload this process carries, when it is a carrier and an upload URL is set, and the
/// history after it. `None` while telemetry is off.
pub(crate) fn report() -> Option<Report> {
    let recorder = Recorder::active()?;
    let outcome = carry_from(&recorder.dir);
    let now = now();
    let mut counters = Counters::read(&recorder.dir);
    counters.roll(Week::of(now.div_euclid(DAY)));
    Some(Report {
        outcome,
        history: counters.history(now),
    })
}

/// Sends the pending week in `dir` now, when this process is a carrier and an upload URL is set.
fn carry_from(dir: &Path) -> Option<Outcome> {
    if !CARRIER.load(Ordering::SeqCst) {
        return None;
    }
    let url = endpoints::telemetry_url()?;
    Some(send(
        dir,
        &url,
        now(),
        rand::rng().random_range(0..WINDOW),
        BUDGET,
    ))
}

/// [`Counters::send`] over the counter file in `dir`, which is written back.
fn send(dir: &Path, url: &str, now: i64, jitter: i64, within: Duration) -> Outcome {
    let mut counters = Counters::read(dir);
    counters.roll(Week::of(now.div_euclid(DAY)));
    let outcome = counters.send(url, now, jitter, within);
    let _ = replace(&dir.join(COUNTERS), &counters.text());
    outcome
}

/// The telemetry folder this process counts into.
struct Recorder {
    dir: PathBuf,
}

impl Recorder {
    /// The data folder's telemetry folder, when consent is `on`, no variable of [`OPT_OUTS`] is
    /// set and the data folder passes its ownership check. `None` records nothing.
    fn active() -> Option<Self> {
        if opt_out(|name| std::env::var_os(name)).is_some() {
            return None;
        }
        let folder = nvs_config::data::current()?;
        let dir = folder.telemetry();
        if !consented(&dir) {
            return None;
        }
        folder.check().ok()?;
        Some(Self { dir })
    }

    /// Adds 1 to `counter` on `day` in the counter file. A file that cannot be written loses the
    /// count and says nothing: telemetry never changes what a command prints.
    fn count(&self, counter: Counter, day: i64) {
        let mut counters = Counters::read(&self.dir);
        counters.count(counter, day);
        let _ = replace(&self.dir.join(COUNTERS), &counters.text());
    }
}

/// What the panic hook needs: where to count and which command is running.
struct Session {
    recorder: Recorder,
    command: &'static str,
    /// Whether `internal-error.<command>` is counted already.
    panicked: AtomicBool,
}

impl Session {
    /// Counts `internal-error.<command>` on `day`, the first time only.
    fn panicked(&self, day: i64) {
        if !self.panicked.swap(true, Ordering::SeqCst) {
            self.recorder
                .count(Counter::new(Kind::InternalError, self.command), day);
        }
    }
}

/// This process's [`Session`], set by [`start`] when telemetry is on.
static SESSION: OnceLock<Session> = OnceLock::new();

/// Counts `invocation.<command>` when telemetry is on, and arms the panic hook for `command`.
/// `carrier` says whether the command may carry an upload. Called once, after the data folder
/// is named.
pub(crate) fn start(command: &str, carrier: bool) {
    CARRIER.store(carrier, Ordering::SeqCst);
    let Some(recorder) = Recorder::active() else {
        return;
    };
    let counter = Counter::new(Kind::Invocation, command);
    recorder.count(counter, today());
    let _ = SESSION.set(Session {
        recorder,
        command: counter.command,
        panicked: AtomicBool::new(false),
    });
}

/// Counts `invocation.unknown` for a command line that names no command, in the data folder
/// `data` names or the default one.
pub(crate) fn unparsed(data: Option<&Path>) {
    let cwd = std::env::current_dir().unwrap_or_default();
    if let Ok(folder) = nvs_config::data::Folder::locate(data, &cwd) {
        let _ = nvs_config::data::set(folder);
    }
    start(UNKNOWN, false);
}

/// Wraps the panic hook in place with one that counts `internal-error.<command>` first. `main`
/// calls it before `nvs_runtime::floor::install_panic_hook`, which then calls this hook only for
/// a panic no served request claimed.
pub(crate) fn install_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if let Some(session) = SESSION.get() {
            session.panicked(today());
        }
        previous(info);
    }));
}

#[cfg(test)]
mod tests {
    use std::io::Read;
    use std::net::TcpListener;
    use std::thread::JoinHandle;

    use clap::CommandFactory;

    use super::*;

    /// Friday 2026-10-23, in ISO week 43.
    fn friday() -> i64 {
        days_from_civil(2026, 10, 23)
    }

    /// A telemetry folder that does not exist yet, inside a scratch directory the caller keeps
    /// alive.
    fn folder() -> (nvs_repo::Scratch, PathBuf) {
        let root = nvs_repo::scratch("telemetry");
        let dir = root.path().join("telemetry");
        (root, dir)
    }

    /// What `verb` printed.
    fn printed(verb: impl FnOnce(&mut Vec<u8>) -> ExitCode) -> String {
        let mut out = Vec::new();
        assert_eq!(verb(&mut out), ExitCode::SUCCESS);
        String::from_utf8(out).expect("text")
    }

    /// An environment with no opt-out and no upload URL.
    fn plain() -> Env {
        Env {
            opt_out: None,
            upload: None,
        }
    }

    #[test]
    fn weeks_follow_iso_8601() {
        assert_eq!(Week::of(friday()).label(), "2026-W43");
        assert_eq!(weekday(friday()), 4);
        // 2021-01-03 is a Sunday in the last week of 2020, and the next day starts week 1.
        assert_eq!(Week::of(days_from_civil(2021, 1, 3)).label(), "2020-W53");
        assert_eq!(Week::of(days_from_civil(2021, 1, 4)).label(), "2021-W01");
        // 2024-12-30 is a Monday, and its Thursday is in 2025.
        assert_eq!(Week::of(days_from_civil(2024, 12, 30)).label(), "2025-W01");
        assert_eq!(Week::parse("2026-W43"), Some(Week::of(friday())));
        assert_eq!(Week::parse("2026-W54"), None);
    }

    #[test]
    fn consent_goes_on_and_off_and_reset_deletes_everything() {
        let (_root, dir) = folder();
        assert!(!consented(&dir));

        let out = printed(|out| on(&dir, &plain(), out));
        assert!(out.starts_with("telemetry: on\n"), "{out}");
        assert!(
            out.contains("Nothing is uploaded, because `NVS_TELEMETRY_URL` is not set."),
            "{out}"
        );
        assert!(consented(&dir));

        Recorder { dir: dir.clone() }.count(Counter::new(Kind::Invocation, "run"), friday());
        let out = printed(|out| off(&dir, out));
        assert!(out.starts_with("telemetry: off\n"), "{out}");
        assert!(out.contains("nvs telemetry reset"), "{out}");
        assert!(!consented(&dir));
        assert!(dir.join(COUNTERS).exists());

        let out = printed(|out| reset(&dir, out));
        assert!(out.starts_with("deleted "), "{out}");
        assert!(!dir.exists());
        let out = printed(|out| reset(&dir, out));
        assert!(out.starts_with("nothing to delete: "), "{out}");
    }

    #[test]
    fn an_opt_out_variable_turns_telemetry_off_and_status_names_it() {
        assert_eq!(opt_out(|_| None), None);
        assert_eq!(opt_out(|name| (name == "CI").then(|| "".into())), None);
        assert_eq!(
            opt_out(|name| (name == "CI").then(|| "true".into())),
            Some("CI")
        );
        assert_eq!(opt_out(|_| Some("1".into())), Some("DO_NOT_TRACK"));

        let (_root, dir) = folder();
        let _ = printed(|out| on(&dir, &plain(), out));
        let env = Env {
            opt_out: Some("DO_NOT_TRACK"),
            upload: Some("http://collector.example/v1".to_owned()),
        };
        let out = printed(|out| status(&dir, &env, friday() * DAY, out));
        assert_eq!(
            out,
            format!(
                "telemetry: off\n\
                 `DO_NOT_TRACK` is set, so nothing is recorded or uploaded while it is set.\n\
                 Without it, telemetry is on.\n\
                 counters: {}\n\
                 upload URL: http://collector.example/v1\n\
                 last upload: never\n\
                 next upload: from 2026-10-26T00:00:00Z\n\
                 uploads accepted: 0, bytes sent: 0\n",
                dir.join(COUNTERS).display()
            )
        );
        let out = printed(|out| status(&dir, &plain(), friday() * DAY, out));
        assert!(out.starts_with("telemetry: on\ncounters: "), "{out}");

        let out = printed(|out| upload(&env, out));
        assert_eq!(
            out,
            "telemetry: off, because `DO_NOT_TRACK` is set. Nothing is uploaded.\n"
        );
        let out = printed(|out| upload(&plain(), out));
        assert_eq!(
            out,
            "Nothing is uploaded, because `NVS_TELEMETRY_URL` is not set.\n"
        );
    }

    #[test]
    fn a_time_prints_as_utc_and_a_week_ends_on_its_monday() {
        assert_eq!(stamp(0), "1970-01-01T00:00:00Z");
        assert_eq!(
            stamp(friday() * DAY + 5 * 3_600 + 12 * 60 + 7),
            "2026-10-23T05:12:07Z"
        );
        assert_eq!(civil(days_from_civil(2024, 2, 29)), (2024, 2, 29));
        assert_eq!(Week::of(friday()).end(), days_from_civil(2026, 10, 26));
        assert_eq!(
            Week::parse("2020-W53").map(Week::end),
            Some(days_from_civil(2021, 1, 4))
        );
        assert_eq!(
            Week::parse("2025-W01").map(Week::end),
            Some(days_from_civil(2025, 1, 6))
        );
    }

    #[test]
    fn a_name_outside_the_command_table_is_unknown() {
        assert_eq!(
            Counter::new(Kind::Invocation, "run").name(),
            "invocation.run"
        );
        assert_eq!(
            Counter::new(Kind::Invocation, "rm -rf ./project").name(),
            "invocation.unknown"
        );
        assert_eq!(
            Counter::parse("internal-error.check"),
            Some(Counter::new(Kind::InternalError, "check"))
        );
        assert_eq!(Counter::parse("invocation./home/somebody"), None);
        assert_eq!(Counter::parse("requests.served"), None);

        // An edited file cannot put a name of its own into a payload.
        let counters = Counters::parse(
            "week 2026-W43\ndays 0000101\ninvocation.run 2\ninvocation.secret-path 9\nroute./admin 4\n",
        );
        let tally = counters.current.as_ref().expect("the week");
        assert_eq!(tally.days, 0b101_0000);
        assert_eq!(
            tally.counts.keys().map(|c| c.name()).collect::<Vec<_>>(),
            ["invocation.run"]
        );

        let unknown = crate::Cli::command().try_get_matches_from(["nvs", "rnu", "app.nvs"]);
        assert!(unknown.is_err());
        let matches = crate::Cli::command()
            .try_get_matches_from(["nvs", "update-check", "--json"])
            .expect("a known command");
        assert_eq!(crate::invoked(&matches), "update-check");
    }

    #[test]
    fn the_command_table_is_every_subcommand_clap_knows() {
        let mut clap: Vec<String> = crate::Cli::command()
            .get_subcommands()
            .map(|command| command.get_name().to_owned())
            .collect();
        clap.sort();
        let mut table: Vec<&str> = crate::COMMAND_NAMES.to_vec();
        table.sort_unstable();
        assert_eq!(clap, table);
        assert!(!table.contains(&UNKNOWN));
    }

    /// The collector's copy of the closed lists, `apps/telemetry/known.toml`, names every command
    /// in the table, `unknown`, and every `os` and `arch` this client can send. It may name more:
    /// a command a later release removes still arrives from the older clients.
    #[test]
    fn the_collectors_known_lists_name_everything_this_client_sends() {
        let known: toml::Table = include_str!("../../../apps/telemetry/known.toml")
            .parse()
            .expect("known.toml parses");
        let list = |key: &str| -> Vec<String> {
            known[key]
                .as_array()
                .expect("a list")
                .iter()
                .map(|value| value.as_str().expect("a string").to_owned())
                .collect()
        };
        let commands = list("commands");
        for name in crate::COMMAND_NAMES.iter().chain([&UNKNOWN]) {
            assert!(
                commands.iter().any(|known| known == name),
                "known.toml lacks command `{name}`"
            );
        }
        let systems = list("os");
        for name in ["linux", "macos", "windows", "freebsd"].map(os) {
            assert!(
                systems.iter().any(|known| known == name),
                "known.toml lacks os `{name}`"
            );
        }
        let processors = list("arch");
        for name in ["x86_64", "aarch64", "riscv64"].map(arch) {
            assert!(
                processors.iter().any(|known| known == name),
                "known.toml lacks arch `{name}`"
            );
        }
    }

    #[test]
    fn days_active_counts_the_days_of_one_week() {
        let monday = friday() - 4;
        let mut counters = Counters::default();
        for day in monday..monday + 7 {
            counters.count(Counter::new(Kind::Invocation, "run"), day);
            counters.count(Counter::new(Kind::Invocation, "check"), day);
        }
        let tally = counters.current.clone().expect("the week");
        assert_eq!(tally.days, 0x7f);
        assert!(counters.payload(&tally).contains(r#""days_active":7,"#));

        // The next Monday starts a new week, and the finished one waits to be sent.
        counters.count(Counter::new(Kind::Invocation, "serve"), monday + 7);
        let pending = counters.pending.clone().expect("the finished week");
        assert_eq!(pending.week.label(), "2026-W43");
        let current = counters.current.clone().expect("the new week");
        assert_eq!(current.week.label(), "2026-W44");
        assert_eq!(current.days, 1);

        // The file reads back as it was written.
        assert_eq!(Counters::parse(&counters.text()), counters);

        // A clock set back drops what it would have to send from the future.
        counters.roll(Week::of(monday - 7));
        assert_eq!(counters.pending, None);
    }

    #[test]
    fn the_payload_is_the_rules_document_and_first_follows_the_upload_count() {
        let mut counters = Counters::default();
        counters.count(Counter::new(Kind::Invocation, "serve"), friday());
        counters.count(Counter::new(Kind::Invocation, "run"), friday());
        counters.count(Counter::new(Kind::Invocation, "run"), friday() + 1);
        counters.count(Counter::new(Kind::InternalError, "check"), friday() + 1);
        let tally = counters.current.clone().expect("the week");
        let expected = |first: bool| {
            format!(
                r#"{{"schema":1,"version":"{VERSION}","os":"{}","arch":"{}","week":"2026-W43","first":{first},"days_active":2,"counters":{{"invocation.run":2,"invocation.serve":1,"internal-error.check":1}}}}"#,
                os(std::env::consts::OS),
                arch(std::env::consts::ARCH),
            )
        };
        assert_eq!(counters.payload(&tally), expected(true));
        let document: serde_json::Value =
            serde_json::from_str(&counters.payload(&tally)).expect("JSON");
        assert_eq!(document["counters"]["invocation.run"], 2);

        counters.uploads = 1;
        counters.bytes = 180;
        let read = Counters::parse(&counters.text());
        assert_eq!(read.payload(&tally), expected(false));

        assert_eq!(os("freebsd"), "other");
        assert_eq!(arch("riscv64"), "other");
    }

    #[test]
    fn show_prints_the_finished_week_before_the_current_one() {
        let (_root, dir) = folder();
        assert_eq!(printed(|out| show(&dir, friday(), out)), "");

        let _ = printed(|out| on(&dir, &plain(), out));
        let recorder = Recorder { dir: dir.clone() };
        recorder.count(Counter::new(Kind::Invocation, "test"), friday());
        let out = printed(|out| show(&dir, friday(), out));
        assert!(out.contains(r#""week":"2026-W43""#), "{out}");
        assert!(out.contains(r#""counters":{"invocation.test":1}"#), "{out}");
        assert_eq!(out.lines().count(), 1);

        recorder.count(Counter::new(Kind::Invocation, "fmt"), friday() + 3);
        let out = printed(|out| show(&dir, friday() + 3, out));
        assert!(out.contains(r#""week":"2026-W43""#), "{out}");
        assert!(out.contains(r#""counters":{"invocation.test":1}"#), "{out}");
    }

    /// A collector on loopback that answers one report with `status` and `body`, and the URL to
    /// reach it. The thread gives back the request's head and body.
    fn collector(status: u16, body: &'static str) -> (String, JoinHandle<(String, Vec<u8>)>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("a loopback port");
        let url = format!(
            "http://{}/v1/report",
            listener.local_addr().expect("the bound address")
        );
        let served = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("the upload connects");
            let mut request = Vec::new();
            let mut buffer = [0_u8; 1024];
            let end = loop {
                if let Some(end) = request.windows(4).position(|w| w == b"\r\n\r\n") {
                    break end + 4;
                }
                let read = stream.read(&mut buffer).expect("the request");
                assert_ne!(read, 0, "the request ended inside its head");
                request.extend_from_slice(&buffer[..read]);
            };
            let head = String::from_utf8_lossy(&request[..end]).into_owned();
            let length: usize = head
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse().ok())?
                })
                .expect("the upload names its length");
            while request.len() < end + length {
                let read = stream.read(&mut buffer).expect("the body");
                assert_ne!(read, 0, "the body ended early");
                request.extend_from_slice(&buffer[..read]);
            }
            let reply = format!(
                "HTTP/1.1 {status} Status\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(reply.as_bytes()).expect("the reply");
            (head, request[end..end + length].to_vec())
        });
        (url, served)
    }

    /// A URL on loopback where nothing listens: a port that was bound and released.
    fn nobody() -> String {
        let port = TcpListener::bind("127.0.0.1:0")
            .and_then(|listener| listener.local_addr())
            .expect("a loopback port")
            .port();
        format!("http://127.0.0.1:{port}/v1/report")
    }

    /// A telemetry folder with consent on and week 2026-W43 finished and not sent, with `now`
    /// inside 2026-W44 after any upload point.
    fn finished() -> (nvs_repo::Scratch, PathBuf, i64) {
        let (root, dir) = folder();
        let _ = printed(|out| on(&dir, &plain(), out));
        let recorder = Recorder { dir: dir.clone() };
        recorder.count(Counter::new(Kind::Invocation, "run"), friday());
        recorder.count(Counter::new(Kind::Invocation, "check"), friday() + 1);
        (root, dir, (friday() + 4) * DAY + WINDOW)
    }

    #[test]
    fn one_upload_sends_the_bytes_show_prints_and_a_week_goes_once() {
        let (_root, dir, now) = finished();
        let shown = printed(|out| show(&dir, now.div_euclid(DAY), out));

        let (url, served) = collector(204, "");
        let outcome = send(&dir, &url, now, 0, BUDGET);
        let (head, body) = served.join().expect("the collector thread");
        assert!(head.starts_with("POST /v1/report "), "{head}");
        let head = head.to_ascii_lowercase();
        assert!(head.contains("content-type: application/json"), "{head}");
        assert!(!head.contains("cookie"), "{head}");
        assert_eq!(format!("{}\n", String::from_utf8_lossy(&body)), shown);

        let bytes = u64::try_from(body.len()).expect("a small body");
        assert_eq!(
            outcome,
            Outcome::Sent {
                week: "2026-W43".to_owned(),
                bytes
            }
        );
        assert_eq!(
            outcome.line(),
            format!("telemetry: sent week 2026-W43, {bytes} bytes")
        );
        let counters = Counters::read(&dir);
        assert_eq!((counters.uploads, counters.bytes), (1, bytes));
        assert_eq!(counters.last, Some(now));
        assert_eq!(counters.pending, None);
        assert_eq!(
            counters.history(now).lines()[1],
            "next upload: from 2026-11-02T00:00:00Z"
        );

        // Nothing is left to send, so a second carrier connects to nobody.
        assert_eq!(send(&dir, &nobody(), now, 0, BUDGET), Outcome::Nothing);
    }

    #[test]
    fn an_empty_week_and_a_week_not_due_are_not_sent() {
        let (_root, dir) = folder();
        let _ = printed(|out| on(&dir, &plain(), out));
        Recorder { dir: dir.clone() }.count(Counter::new(Kind::Invocation, "run"), friday());
        // The week is still running.
        assert_eq!(
            send(&dir, &nobody(), friday() * DAY, 0, BUDGET),
            Outcome::Nothing
        );

        // The week ended, and its point is one hour into the next Monday.
        let monday = (friday() + 3) * DAY;
        let outcome = send(&dir, &nobody(), monday + 60, 3_600, BUDGET);
        assert_eq!(
            outcome.line(),
            "telemetry: week 2026-W43 is sent from 2026-10-26T01:00:00Z"
        );
        // The point is drawn once, so a later carrier keeps it.
        let outcome = send(&dir, &nobody(), monday + 120, 7_200, BUDGET);
        assert_eq!(outcome.result(), "not-due");
        assert_eq!(Counters::read(&dir).due, Some(monday + 3_600));

        // A second upload inside the week of the last one waits for that week to end.
        let mut counters = Counters::read(&dir);
        counters.last = Some(monday + 30);
        let _ = replace(&dir.join(COUNTERS), &counters.text());
        let outcome = send(&dir, &nobody(), monday + 2 * 3_600, 3_600, BUDGET);
        assert_eq!(
            outcome,
            Outcome::NotDue {
                week: "2026-W43".to_owned(),
                at: monday + 7 * DAY
            }
        );
    }

    #[test]
    fn a_collector_that_never_answers_costs_the_budget_and_keeps_the_week() {
        let (_root, dir, now) = finished();
        let listener = TcpListener::bind("127.0.0.1:0").expect("a loopback port");
        let url = format!(
            "http://{}/v1/report",
            listener.local_addr().expect("the bound address")
        );
        let within = Duration::from_millis(500);
        let started = Instant::now();
        let outcome = send(&dir, &url, now, 0, within);
        let took = started.elapsed();
        drop(listener);
        assert_eq!(
            outcome.line(),
            "telemetry: could not send (timeout after 500ms); kept for next time"
        );
        assert!(took < within + Duration::from_secs(2), "{took:?}");
        assert!(Counters::read(&dir).pending.is_some());

        let outcome = send(&dir, &nobody(), now, 0, BUDGET);
        assert_eq!(outcome.result(), "kept", "{}", outcome.line());
        assert!(
            outcome.line().ends_with("; kept for next time"),
            "{}",
            outcome.line()
        );
        assert_eq!(Counters::read(&dir).uploads, 0);
    }

    #[test]
    fn the_collectors_answer_keeps_or_drops_the_week() {
        for (status, body, line) in [
            (
                429,
                "",
                "telemetry: could not send (the collector returned 429); kept for next time",
            ),
            (
                500,
                "",
                "telemetry: could not send (the collector returned 500); kept for next time",
            ),
            (
                503,
                "",
                "telemetry: could not send (the collector returned 503); kept for next time",
            ),
            (
                418,
                "",
                "telemetry: could not send (the collector returned 418); kept for next time",
            ),
            (
                400,
                "week\r\nmore",
                "telemetry: refused by the collector (400: week); dropped",
            ),
            (
                400,
                "\u{1b}[2Jweek",
                "telemetry: refused by the collector (400: [2Jweek); dropped",
            ),
            (
                413,
                "",
                "telemetry: refused by the collector (413); dropped",
            ),
            (
                415,
                "",
                "telemetry: refused by the collector (415); dropped",
            ),
        ] {
            let (_root, dir, now) = finished();
            let (url, served) = collector(status, body);
            let outcome = send(&dir, &url, now, 0, BUDGET);
            served.join().expect("the collector thread");
            assert_eq!(outcome.line(), line);
            let counters = Counters::read(&dir);
            assert_eq!(counters.uploads, 0, "{status}");
            assert_eq!(
                counters.pending.is_some(),
                outcome.result() == "kept",
                "{status}"
            );
        }
    }

    #[test]
    fn only_a_carrier_uploads() {
        let (_root, dir, _) = finished();
        CARRIER.store(false, Ordering::SeqCst);
        assert_eq!(carry_from(&dir), None);
    }

    #[test]
    fn a_panic_counts_one_internal_error() {
        let (_root, dir) = folder();
        let _ = printed(|out| on(&dir, &plain(), out));
        let session = Session {
            recorder: Recorder { dir: dir.clone() },
            command: Counter::new(Kind::Invocation, "check").command,
            panicked: AtomicBool::new(false),
        };
        session.panicked(friday());
        session.panicked(friday());
        let counters = Counters::read(&dir);
        let tally = counters.current.expect("the week");
        assert_eq!(
            tally
                .counts
                .get(&Counter::new(Kind::InternalError, "check")),
            Some(&1)
        );
        assert_eq!(tally.counts.len(), 1);
    }
}
