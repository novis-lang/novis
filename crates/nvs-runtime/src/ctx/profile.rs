//! `rule:testing/debug-surface`'s profiler, behind [`DebugFlags::PROFILE`]: the call probes time
//! every call, and a [`Profiler`] writes what they time as one of
//! `rule:observability/speedscope-timeline-export`'s two exports.
//!
//! A [`Profiler`] is the file and what every context profiling into it shares. A [`ProfileTrack`]
//! is one context's own part of it: the calls that context has in flight, and the events it has
//! written. A track is never shared, because two contexts' calls interleave and would not nest.
//!
//! - **speedscope** is an evented timeline, one profile per track. The first track writes its
//!   events straight into the file. Every other track writes them to a part file beside it, which
//!   is copied in once the first track has closed and then deleted. Only a host-installed profiler
//!   ([`Ctx::set_run_profile`]) has more than one track, so a profile `Core\Debug::startProfile`
//!   opened writes the one file its grant names and nothing beside it.
//! - **Callgrind** is self and inclusive time per function, and per caller and callee. Each track
//!   adds its totals into the profiler's when it closes, and [`Profiler::finish`] writes the file.
//!
//! A track closes when it is dropped, so a context that ends with a profile running closes it.
//! A write that fails stops that track's writing and is kept until [`Profiler::finish`] returns
//! it, because a profile observes a run and never stops one.
//!
//! **What it spends:** per track, one entry per call in flight and one cached index per distinct
//! function it called, and under Callgrind one total per function and per caller and callee pair.
//! Per profiler, one name per distinct function and one entry per track waiting to be copied in.
//! The events are in the file, so the process does not grow with the run.

use super::*;

use std::collections::HashMap;
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

/// The schema URL a speedscope file names, which is what the viewer checks.
pub const SPEEDSCOPE_SCHEMA: &str = "https://www.speedscope.app/file-format-schema.json";

/// Which of the two exports a [`Profiler`] writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProfileFormat {
    /// speedscope's evented JSON: a timeline per track.
    Speedscope,
    /// Callgrind's text format: self and inclusive time per function.
    Callgrind,
}

/// One profile file and what its tracks share. See the module doc.
#[derive(Debug)]
pub struct Profiler {
    format: ProfileFormat,
    path: PathBuf,
    /// What the profile is of, written into the file's header.
    name: Box<str>,
    shared: Mutex<Shared>,
}

/// What a [`Profiler`]'s tracks write into, under its lock.
#[derive(Debug)]
struct Shared {
    out: Option<BufWriter<std::fs::File>>,
    names: Vec<Box<str>>,
    index: HashMap<Box<str>, u32>,
    /// How many tracks have been opened, which numbers each one.
    tracks: u32,
    /// Whether the first track still writes into the file directly.
    direct_open: bool,
    /// Whether a profile has been written into the `profiles` array.
    wrote_profile: bool,
    /// Part files waiting for the direct track to close.
    pending: Vec<Part>,
    /// Callgrind's totals, by function.
    costs: HashMap<u32, Cost>,
    /// Callgrind's totals, by caller and callee.
    calls: HashMap<(u32, u32), Cost>,
    failed: Option<std::io::Error>,
    finished: bool,
}

/// A closed track's events, in a part file, with what its profile header needs.
#[derive(Debug)]
struct Part {
    name: Box<str>,
    path: PathBuf,
    start: u64,
    end: u64,
}

/// Nanoseconds and a call count.
#[derive(Clone, Copy, Debug, Default)]
struct Cost {
    calls: u64,
    /// Self time for a function, and inclusive time for a caller and callee pair.
    nanos: u64,
    /// Inclusive time for a function.
    inclusive: u64,
}

impl Profiler {
    /// Creates `path`, or empties it, for a profile in `format`. `name` is what the profile is
    /// of, such as the program's file.
    ///
    /// # Errors
    ///
    /// What the operating system returned for the create, or for the header's write.
    pub fn create(path: &Path, format: ProfileFormat, name: &str) -> std::io::Result<Arc<Self>> {
        let mut out = BufWriter::new(std::fs::File::create(path)?);
        if format == ProfileFormat::Speedscope {
            out.write_all(br#"{"$schema":""#)?;
            out.write_all(SPEEDSCOPE_SCHEMA.as_bytes())?;
            out.write_all(br#"","exporter":"nvs","name":""#)?;
            write_json_text(&mut out, name)?;
            out.write_all(br#"","activeProfileIndex":0,"profiles":["#)?;
        }
        Ok(Arc::new(Self {
            format,
            path: path.to_path_buf(),
            name: name.into(),
            shared: Mutex::new(Shared {
                out: Some(out),
                names: Vec::new(),
                index: HashMap::new(),
                tracks: 0,
                direct_open: false,
                wrote_profile: false,
                pending: Vec::new(),
                costs: HashMap::new(),
                calls: HashMap::new(),
                failed: None,
                finished: false,
            }),
        }))
    }

    /// The file this profile writes.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The format this profile writes.
    #[must_use]
    pub fn format(&self) -> ProfileFormat {
        self.format
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Shared> {
        self.shared.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Opens a track named `name`, for one context. The first track writes straight into the file
    /// and every later one into a part file of its own.
    #[must_use]
    pub fn track(self: &Arc<Self>, name: &str) -> ProfileTrack {
        let mut shared = self.lock();
        shared.tracks += 1;
        let number = shared.tracks;
        let direct = number == 1;
        if direct {
            shared.direct_open = true;
        }
        drop(shared);
        let mut track = ProfileTrack {
            profiler: Arc::clone(self),
            name: name.into(),
            number,
            frames: HashMap::new(),
            open: Vec::new(),
            out: TrackOut::None,
            costs: HashMap::new(),
            calls: HashMap::new(),
            first: None,
            last: 0,
            written: false,
            failed: None,
        };
        if self.format == ProfileFormat::Speedscope {
            track.out = if direct {
                TrackOut::Direct
            } else {
                match std::fs::File::create(self.part_path(number)) {
                    Ok(file) => TrackOut::Part(BufWriter::new(file)),
                    Err(err) => {
                        track.failed = Some(err);
                        TrackOut::None
                    }
                }
            };
        }
        track
    }

    /// A child context's track, named for the track of the context that started it.
    #[must_use]
    pub fn child_track(self: &Arc<Self>, parent: Option<&ProfileTrack>) -> ProfileTrack {
        let number = self.lock().tracks + 1;
        let parent = parent.map_or(&*self.name, |track| &*track.name);
        self.track(&format!("{parent} > {number}"))
    }

    fn part_path(&self, number: u32) -> PathBuf {
        let mut name = self.path.as_os_str().to_owned();
        name.push(format!(".{number}.part"));
        PathBuf::from(name)
    }

    /// The global index of the function `label`, adding it when it is new.
    fn frame(&self, label: &str) -> u32 {
        let mut shared = self.lock();
        if let Some(&index) = shared.index.get(label) {
            return index;
        }
        let index = u32::try_from(shared.names.len()).unwrap_or(u32::MAX);
        shared.names.push(label.into());
        shared.index.insert(label.into(), index);
        index
    }

    /// Writes what is left and closes the file. A track still open then is not in it. A second
    /// call does nothing.
    ///
    /// # Errors
    ///
    /// The first write that failed while the profile ran, or the final write's own failure.
    pub fn finish(&self) -> std::io::Result<()> {
        let mut shared = self.lock();
        if shared.finished {
            return Ok(());
        }
        shared.finished = true;
        let written = match self.format {
            ProfileFormat::Speedscope => {
                shared.direct_open = false;
                shared.copy_pending();
                shared.write_speedscope_tail()
            }
            ProfileFormat::Callgrind => shared.write_callgrind(&self.name),
        };
        let flushed = written.and_then(|()| match shared.out.take() {
            Some(mut out) => out.flush(),
            None => Ok(()),
        });
        match shared.failed.take() {
            Some(failed) => Err(failed),
            None => flushed,
        }
    }
}

impl Drop for Profiler {
    fn drop(&mut self) {
        // The error has nobody to go to here; a host that wants it calls `finish` first.
        let _ = self.finish();
    }
}

impl Shared {
    fn keep_failure(&mut self, result: std::io::Result<()>) {
        if let Err(err) = result
            && self.failed.is_none()
        {
            self.failed = Some(err);
        }
    }

    /// Writes one profile's opening, up to its `events` array. Its `endValue` follows the array,
    /// because the end is known only once the events are written.
    fn open_profile(&mut self, name: &str, start: u64) -> std::io::Result<()> {
        let comma = std::mem::replace(&mut self.wrote_profile, true);
        let Some(out) = &mut self.out else {
            return Ok(());
        };
        if comma {
            out.write_all(b",")?;
        }
        out.write_all(br#"{"type":"evented","name":""#)?;
        write_json_text(out, name)?;
        write!(
            out,
            r#"","unit":"nanoseconds","startValue":{start},"events":["#
        )
    }

    /// Writes one profile's close, after its last event.
    fn close_profile(&mut self, end: u64) -> std::io::Result<()> {
        match &mut self.out {
            Some(out) => write!(out, r#"],"endValue":{end}}}"#),
            None => Ok(()),
        }
    }

    /// Copies every waiting part file in as a profile of its own, and deletes it.
    fn copy_pending(&mut self) {
        for part in std::mem::take(&mut self.pending) {
            let copied = self.copy_part(&part);
            self.keep_failure(copied);
            let _ = std::fs::remove_file(&part.path);
        }
    }

    fn copy_part(&mut self, part: &Part) -> std::io::Result<()> {
        self.open_profile(&part.name, part.start)?;
        if let Some(out) = &mut self.out {
            let mut file = std::fs::File::open(&part.path)?;
            std::io::copy(&mut file, out)?;
        }
        self.close_profile(part.end)
    }

    fn write_speedscope_tail(&mut self) -> std::io::Result<()> {
        let Some(out) = &mut self.out else {
            return Ok(());
        };
        out.write_all(br#"],"shared":{"frames":["#)?;
        for (at, name) in self.names.iter().enumerate() {
            if at > 0 {
                out.write_all(b",")?;
            }
            out.write_all(br#"{"name":""#)?;
            write_json_text(out, name)?;
            out.write_all(br#""}"#)?;
        }
        out.write_all(b"]}}\n")
    }

    /// Callgrind's file: a header, then one `fn=` block per function with its self time and one
    /// `cfn=` and `calls=` pair per function it called, with that call's inclusive time.
    fn write_callgrind(&mut self, cmd: &str) -> std::io::Result<()> {
        let Some(out) = &mut self.out else {
            return Ok(());
        };
        let total: u64 = self.costs.values().map(|cost| cost.nanos).sum();
        writeln!(out, "# callgrind format")?;
        writeln!(out, "version: 1")?;
        writeln!(out, "creator: nvs")?;
        writeln!(out, "cmd: {cmd}")?;
        writeln!(out, "positions: line")?;
        writeln!(out, "events: Nanoseconds")?;
        writeln!(out, "summary: {total}")?;
        let mut functions: Vec<_> = self.costs.iter().map(|(&f, &c)| (f, c)).collect();
        functions.sort_unstable_by_key(|&(f, _)| f);
        let mut calls: Vec<_> = self.calls.iter().map(|(&e, &c)| (e, c)).collect();
        calls.sort_unstable_by_key(|&(e, _)| e);
        let name = |f: u32| self.names.get(f as usize).map_or("?", |n| &**n);
        for (function, cost) in functions {
            writeln!(out)?;
            writeln!(out, "fn={}", name(function))?;
            writeln!(out, "0 {}", cost.nanos)?;
            for &((_, callee), call) in calls.iter().filter(|((caller, _), _)| *caller == function)
            {
                writeln!(out, "cfn={}", name(callee))?;
                writeln!(out, "calls={} 0", call.calls)?;
                writeln!(out, "0 {}", call.nanos)?;
            }
        }
        writeln!(out)?;
        writeln!(out, "totals: {total}")
    }
}

/// Where a track's speedscope events go.
#[derive(Debug)]
enum TrackOut {
    /// Nowhere: a Callgrind track, or a part file that could not be created.
    None,
    /// Straight into the profiler's file, under its lock.
    Direct,
    /// Into a part file of the track's own.
    Part(BufWriter<std::fs::File>),
}

/// A call in flight on a track.
#[derive(Debug)]
struct Open {
    frame: u32,
    start: u64,
    /// The inclusive time of the calls it made, which its self time leaves out.
    children: u64,
}

/// One context's part of a [`Profiler`]. See the module doc.
#[derive(Debug)]
pub struct ProfileTrack {
    profiler: Arc<Profiler>,
    name: Box<str>,
    number: u32,
    /// The profiler's index of each function this track has called.
    frames: HashMap<Box<str>, u32>,
    open: Vec<Open>,
    out: TrackOut,
    costs: HashMap<u32, Cost>,
    calls: HashMap<(u32, u32), Cost>,
    /// The first event's time, once there is one.
    first: Option<u64>,
    /// The latest event's time. An event is never written before it, so `at` never decreases.
    last: u64,
    /// Whether an event has been written, so the next one is preceded by a comma.
    written: bool,
    failed: Option<std::io::Error>,
}

impl ProfileTrack {
    /// The profiler this track writes into.
    #[must_use]
    pub fn profiler(&self) -> &Arc<Profiler> {
        &self.profiler
    }

    /// `reading` in nanoseconds, moved up to the latest event's time.
    fn at(&mut self, reading: Duration) -> u64 {
        let at = u64::try_from(reading.as_nanos())
            .unwrap_or(u64::MAX)
            .max(self.last);
        self.first.get_or_insert(at);
        self.last = at;
        at
    }

    /// A call to `label` begins now.
    pub fn enter(&mut self, label: &str) {
        self.enter_at(label, trace_clock());
    }

    /// The innermost call in flight returns now. An exit whose entry came before the profile
    /// started has nothing to close and is not counted.
    pub fn exit(&mut self) {
        self.exit_at(trace_clock());
    }

    /// Something that is not a call, such as a collector pause, ran from `start` to `end`. It
    /// is a call of its own inside the call in flight, so its time is not that call's self time.
    pub fn window(&mut self, label: &str, start: Duration, end: Duration) {
        self.enter_at(label, start);
        self.exit_at(end.max(start));
    }

    fn enter_at(&mut self, label: &str, reading: Duration) {
        let frame = match self.frames.get(label) {
            Some(&frame) => frame,
            None => {
                let frame = self.profiler.frame(label);
                self.frames.insert(label.into(), frame);
                frame
            }
        };
        let at = self.at(reading);
        self.open.push(Open {
            frame,
            start: at,
            children: 0,
        });
        self.event(b'O', frame, at);
    }

    fn exit_at(&mut self, reading: Duration) {
        if self.open.is_empty() {
            return;
        }
        let at = self.at(reading);
        self.close_one(at);
    }

    fn close_one(&mut self, at: u64) {
        let Some(open) = self.open.pop() else {
            return;
        };
        let inclusive = at.saturating_sub(open.start);
        if self.profiler.format == ProfileFormat::Callgrind {
            let cost = self.costs.entry(open.frame).or_default();
            cost.calls += 1;
            cost.nanos += inclusive.saturating_sub(open.children);
            cost.inclusive += inclusive;
            if let Some(caller) = self.open.last() {
                let call = self.calls.entry((caller.frame, open.frame)).or_default();
                call.calls += 1;
                call.nanos += inclusive;
            }
        }
        if let Some(caller) = self.open.last_mut() {
            caller.children += inclusive;
        }
        self.event(b'C', open.frame, at);
    }

    fn event(&mut self, kind: u8, frame: u32, at: u64) {
        if self.failed.is_some() {
            return;
        }
        let line = |out: &mut dyn std::io::Write, comma: bool| {
            if comma {
                out.write_all(b",")?;
            }
            write!(
                out,
                r#"{{"type":"{}","frame":{frame},"at":{at}}}"#,
                char::from(kind)
            )
        };
        let comma = std::mem::replace(&mut self.written, true);
        let written = match &mut self.out {
            TrackOut::None => Ok(()),
            TrackOut::Part(out) => line(out, comma),
            TrackOut::Direct => {
                let mut shared = self.profiler.lock();
                // The direct track's profile opens at its first event.
                let opening = if comma {
                    Ok(())
                } else {
                    shared.open_profile(&self.name, at)
                };
                opening.and_then(|()| match &mut shared.out {
                    Some(out) => line(out, comma),
                    None => Ok(()),
                })
            }
        };
        if let Err(err) = written {
            self.failed = Some(err);
        }
    }

    /// Closes every call still in flight now, and hands what the track wrote to its profiler.
    fn close(&mut self) {
        if !self.open.is_empty() {
            let at = self.at(trace_clock());
            while !self.open.is_empty() {
                self.close_one(at);
            }
        }
        let out = std::mem::replace(&mut self.out, TrackOut::None);
        let costs = std::mem::take(&mut self.costs);
        let calls = std::mem::take(&mut self.calls);
        let mut shared = self.profiler.lock();
        for (frame, cost) in costs {
            let total = shared.costs.entry(frame).or_default();
            total.calls += cost.calls;
            total.nanos += cost.nanos;
            total.inclusive += cost.inclusive;
        }
        for (pair, call) in calls {
            let total = shared.calls.entry(pair).or_default();
            total.calls += call.calls;
            total.nanos += call.nanos;
        }
        match out {
            TrackOut::None => {}
            TrackOut::Direct => {
                if self.written && self.failed.is_none() {
                    let closed = shared.close_profile(self.last);
                    shared.keep_failure(closed);
                }
                shared.direct_open = false;
                shared.copy_pending();
            }
            TrackOut::Part(mut out) => {
                let path = self.profiler.part_path(self.number);
                let flushed = out.flush();
                drop(out);
                if self.written && flushed.is_ok() && !shared.finished {
                    shared.pending.push(Part {
                        name: self.name.clone(),
                        path,
                        start: self.first.unwrap_or(self.last),
                        end: self.last,
                    });
                    if !shared.direct_open {
                        shared.copy_pending();
                    }
                } else {
                    shared.keep_failure(flushed);
                    let _ = std::fs::remove_file(path);
                }
            }
        }
        if let Some(failed) = self.failed.take() {
            shared.keep_failure(Err(failed));
        }
    }
}

impl Drop for ProfileTrack {
    fn drop(&mut self) {
        self.close();
    }
}
