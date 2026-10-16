//! The load probe: what `nvs serve` saturates at on this machine, and whether
//! every answer it gave was the answer to the request that asked.
//!
//! `bun nv bench-load` is what runs this — it compiles the probe, starts the
//! server, walks the concurrency list and writes the record — and
//! `tools/nv/cmd/bench-load.ts`'s `# What this leg is` owns why the leg exists
//! and how it differs from the other two. What is *here* is the client: the
//! token every request carries, the two ways load is applied, and the checks a
//! response has to pass.
//!
//! # A throughput figure over one constant response measures less than it looks
//!
//! Drive a server with the same `GET /` on every connection and every answer is
//! byte-identical, so an answer delivered to the wrong connection is
//! indistinguishable from the right one. The requests-per-second figure is real
//! and the correctness claim behind it is empty — and a server that crosses two
//! responses under load is worse than a slow one, because nothing downstream
//! can detect it either.
//!
//! So every request here carries a token belonging to one connection and one
//! position in that connection's sequence, and the body it gets back has to
//! name that token. The case is `benches/serve/echo.nvs`, whose whole body is
//! `id=<token>\n`. Three failures are then visible that a constant body hides:
//! a response delivered on the wrong connection, a response answering an
//! earlier request on the right connection — a one-off skew that would
//! otherwise persist unseen for the rest of the run — and bytes arriving after
//! a response has been framed off, which is a second answer to a request that
//! asked once.
//!
//! **The check is guarded by its own tests.** A verifier that has quietly
//! stopped comparing is a green run that means nothing, so
//! `a_constant_body_fails_every_request` holds this probe to rejecting the
//! `hello, world` case it would otherwise pass silently.
//!
//! # Two modes, because "concurrent" is two different questions
//!
//! [`Mode::ClosedLoop`] is throughput: N connections, one outstanding request
//! each, for a fixed wall time. One thread per connection, which is what caps
//! it — past a couple of thousand the generator is measuring the operating
//! system's scheduler — so it refuses a count it could not drive honestly and
//! names the other mode.
//!
//! [`Mode::InFlight`] is simultaneity: every request written before any answer
//! is read, so all N are being served together rather than N-at-a-time. One
//! thread sweeping non-blocking sockets, because ten thousand threads would
//! measure this program.
//!
//! # What a run does not measure
//!
//! The generator shares the machine with the server, so both numbers are a
//! floor rather than the hardware's ceiling. Nothing here crosses a network
//! interface, negotiates TLS or passes a proxy, and the case does no work of
//! its own — which is the point when the question is what the *server* costs,
//! and the reason a figure from this leg is never a claim about an application.
//!
//! # What a client-side limit looks like, and why it is never a server figure
//!
//! Connections cost the client an ephemeral port and a descriptor, and both run
//! out long before a server does. A run that could not open what it was asked
//! for reports `connections_opened` below `connections_requested` and sets
//! `client_limited`, so the number is never read as the server's ceiling. The
//! driver raises the descriptor limit before starting this process; the port
//! range is the operating system's, and a run that exhausts it says so.

#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "this binary's whole output is a record on stdout and notes on stderr"
)]

use std::fmt::Write as _;
use std::io::{ErrorKind, Read, Write};
use std::net::TcpStream;
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Barrier, Mutex};
use std::time::{Duration, Instant};

use clap::{Parser, ValueEnum};

/// Past this, one thread per connection is measuring the scheduler rather than
/// the server, so [`Mode::ClosedLoop`] refuses instead of reporting it.
const MAX_CLOSED_LOOP: usize = 2048;

/// Latency buckets, one per power of two microseconds.
const BUCKETS: usize = 40;

/// How many mismatches are described in full before the record only counts them.
const NOTES: usize = 10;

#[derive(Debug, Parser)]
#[command(
    about = "Drive `nvs serve` and verify every answer belongs to the request that asked",
    long_about = None
)]
struct Args {
    /// The `host:port` to drive.
    #[arg(long)]
    target: String,

    /// Throughput at a fixed concurrency, or simultaneity at a fixed width.
    #[arg(long, value_enum)]
    mode: Mode,

    /// Connections to open.
    #[arg(long)]
    connections: usize,

    /// Closed-loop only: how long to drive, in seconds.
    #[arg(long, default_value_t = 10)]
    seconds: u64,

    /// In-flight only: how many times to put every request out at once.
    #[arg(long, default_value_t = 3)]
    rounds: usize,

    /// A response that takes longer than this is a stall, not a slow answer.
    #[arg(long, default_value_t = 5_000)]
    timeout_ms: u64,
}

/// Which question a run is asking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum Mode {
    /// N connections, one outstanding request each, for a fixed wall time.
    ClosedLoop,
    /// N requests all written before any answer is read.
    InFlight,
}

fn main() -> ExitCode {
    let args = Args::parse();
    if args.connections == 0 {
        eprintln!("--connections must be at least 1");
        return ExitCode::from(2);
    }
    if args.mode == Mode::ClosedLoop && args.connections > MAX_CLOSED_LOOP {
        eprintln!(
            "--connections {} is past this mode's {MAX_CLOSED_LOOP}: one thread per connection \
             stops measuring the server before then. `--mode in-flight` drives more than that on \
             one thread.",
            args.connections
        );
        return ExitCode::from(2);
    }
    let outcome = match args.mode {
        Mode::ClosedLoop => closed_loop(&args),
        Mode::InFlight => in_flight(&args),
    };
    println!("{}", outcome.record);
    if outcome.wrong > 0 {
        eprintln!(
            "{} answer(s) were not the answer to the request that asked",
            outcome.wrong
        );
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}

/// A finished run: the JSON record, and how many answers were wrong.
#[derive(Debug)]
struct Outcome {
    record: String,
    wrong: u64,
}

// ---------------------------------------------------------------------------
// The token, the request it names, and the answer it demands
// ---------------------------------------------------------------------------

/// The token one request carries: this run, this connection, this position.
///
/// The process id is in it so that an answer held over on a reused port from an
/// earlier run could never be mistaken for one of this run's.
fn token(run: u32, conn: usize, seq: u64) -> String {
    format!("{run:x}-{conn:x}-{seq:x}")
}

/// The request that asks for one token.
fn request_for(token: &str) -> String {
    format!(
        "GET /?n={token} HTTP/1.1\r\nHost: probe\r\nAccept: */*\r\nConnection: keep-alive\r\n\r\n"
    )
}

/// The body `benches/serve/echo.nvs` owes that token, exactly.
fn body_for(token: &str) -> String {
    format!("id={token}\n")
}

/// One response, framed off the stream.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Response {
    status: [u8; 3],
    body: Vec<u8>,
}

/// Why one answer was not the answer that was asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Wrong {
    /// A status other than `200`.
    Status,
    /// A body naming a different request, or no request at all.
    Body,
}

/// Whether this answer belongs to the request that asked for `token`.
fn verify(token: &str, answer: &Response) -> Result<(), Wrong> {
    if answer.status != *b"200" {
        return Err(Wrong::Status);
    }
    if answer.body != body_for(token).as_bytes() {
        return Err(Wrong::Body);
    }
    Ok(())
}

/// Takes one whole response off the front of `held`, if all of it has arrived.
///
/// `Content-Length` is the only framing this leg's case produces, and a
/// response without one is a header set this probe should not silently accept —
/// so an absent length frames a zero-length body and fails [`verify`] rather
/// than being guessed at.
fn take_response(held: &mut Vec<u8>) -> Option<Response> {
    let end = held.windows(4).position(|window| window == b"\r\n\r\n")?;
    let head = &held[..end];
    if head.len() < 12 {
        return None;
    }
    let len = content_length(head).unwrap_or(0);
    let total = end + 4 + len;
    if held.len() < total {
        return None;
    }
    let mut status = [0u8; 3];
    status.copy_from_slice(&head[9..12]);
    let body = held[end + 4..total].to_vec();
    held.drain(..total);
    Some(Response { status, body })
}

/// The `Content-Length` a head declares.
///
/// A line with no colon is skipped rather than ending the search: the first one
/// is always the status line, so a `?` here would answer `None` for every
/// response ever framed — which the tests below caught, and is why they frame a
/// whole response rather than only asking whether a header parses.
fn content_length(head: &[u8]) -> Option<usize> {
    let text = String::from_utf8_lossy(head);
    for line in text.split("\r\n") {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        if name.trim().eq_ignore_ascii_case("content-length") {
            return value.trim().parse().ok();
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Opening connections, and saying so when the client is what ran out
// ---------------------------------------------------------------------------

/// A connection with both deadlines set, or the reason there is not one.
fn open(target: &str, timeout: Duration, nonblocking: bool) -> std::io::Result<TcpStream> {
    let sock = TcpStream::connect(target)?;
    sock.set_nodelay(true)?;
    if nonblocking {
        sock.set_nonblocking(true)?;
    } else {
        sock.set_read_timeout(Some(timeout))?;
        sock.set_write_timeout(Some(timeout))?;
    }
    Ok(sock)
}

/// What a run managed to open, and why it stopped if it stopped early.
#[derive(Debug, Default)]
struct Opened {
    refused_at: Option<usize>,
    reason: Option<String>,
}

impl Opened {
    /// The record's two fields for a client that ran out of ports or
    /// descriptors before the server ran out of anything.
    fn fields(&self, requested: usize, opened: usize) -> String {
        let limited = opened < requested;
        let reason = match (&self.reason, self.refused_at) {
            (Some(reason), Some(at)) => json_string(&format!("connection {at}: {reason}")),
            _ => "null".to_owned(),
        };
        format!(
            "\"connections_requested\": {requested}, \"connections_opened\": {opened}, \
             \"client_limited\": {limited}, \"connect_error\": {reason}"
        )
    }
}

// ---------------------------------------------------------------------------
// Closed loop: throughput at one concurrency
// ---------------------------------------------------------------------------

/// Every counter a closed-loop run accumulates, shared across its threads.
#[derive(Debug)]
struct Counts {
    verified: AtomicU64,
    bad_status: AtomicU64,
    bad_body: AtomicU64,
    leftover: AtomicU64,
    stalls: AtomicU64,
    errors: AtomicU64,
    lat_ns: AtomicU64,
    max_ns: AtomicU64,
    hist: Vec<AtomicU64>,
    notes: Mutex<Vec<String>>,
}

impl Counts {
    fn new() -> Self {
        Self {
            verified: AtomicU64::new(0),
            bad_status: AtomicU64::new(0),
            bad_body: AtomicU64::new(0),
            leftover: AtomicU64::new(0),
            stalls: AtomicU64::new(0),
            errors: AtomicU64::new(0),
            lat_ns: AtomicU64::new(0),
            max_ns: AtomicU64::new(0),
            hist: (0..BUCKETS).map(|_| AtomicU64::new(0)).collect(),
            notes: Mutex::new(Vec::new()),
        }
    }

    fn note(&self, line: String) {
        let mut notes = self.notes.lock().expect("the notes lock is never poisoned");
        if notes.len() < NOTES {
            notes.push(line);
        }
    }
}

/// N connections, one outstanding request each, for `--seconds`.
fn closed_loop(args: &Args) -> Outcome {
    let run = std::process::id();
    let timeout = Duration::from_millis(args.timeout_ms);
    let counts = Arc::new(Counts::new());
    let stop = Arc::new(AtomicBool::new(false));
    let opened_count = Arc::new(AtomicU64::new(0));
    let opening = Arc::new(Mutex::new(Opened::default()));
    // Every connection is open, and one exchange done, before the clock starts:
    // a handshake and a cold first request are not what is being measured.
    let ready = Arc::new(Barrier::new(args.connections + 1));
    let go = Arc::new(Barrier::new(args.connections + 1));

    let mut threads = Vec::with_capacity(args.connections);
    for conn in 0..args.connections {
        let target = args.target.clone();
        let counts = Arc::clone(&counts);
        let stop = Arc::clone(&stop);
        let opened_count = Arc::clone(&opened_count);
        let opening = Arc::clone(&opening);
        let (ready, go) = (Arc::clone(&ready), Arc::clone(&go));
        threads.push(std::thread::spawn(move || {
            let mut sock = match open(&target, timeout, false) {
                Ok(sock) => sock,
                Err(err) => {
                    let mut opening = opening.lock().expect("the opening lock is never poisoned");
                    if opening.refused_at.is_none_or(|at| conn < at) {
                        opening.refused_at = Some(conn);
                        opening.reason = Some(err.to_string());
                    }
                    drop(opening);
                    ready.wait();
                    go.wait();
                    return;
                }
            };
            opened_count.fetch_add(1, Ordering::Relaxed);
            let mut buf = vec![0u8; 16 * 1024];
            let mut held: Vec<u8> = Vec::with_capacity(16 * 1024);
            let mut seq: u64 = 0;
            let _ = exchange(&mut sock, &token(run, conn, seq), &mut buf, &mut held);
            ready.wait();
            go.wait();
            let mut local = vec![0u64; BUCKETS];
            let (mut sum, mut peak) = (0u64, 0u64);
            while !stop.load(Ordering::Relaxed) {
                seq += 1;
                let want = token(run, conn, seq);
                let at = Instant::now();
                match exchange(&mut sock, &want, &mut buf, &mut held) {
                    Ok(answered) => {
                        let ns = u64::try_from(at.elapsed().as_nanos()).unwrap_or(u64::MAX);
                        sum += ns;
                        peak = peak.max(ns);
                        local[bucket(ns)] += 1;
                        match verify(&want, &answered.response) {
                            Ok(()) => {
                                counts.verified.fetch_add(1, Ordering::Relaxed);
                            }
                            Err(Wrong::Status) => {
                                counts.bad_status.fetch_add(1, Ordering::Relaxed);
                                counts.note(format!(
                                    "{want} was answered {}",
                                    String::from_utf8_lossy(&answered.response.status)
                                ));
                            }
                            Err(Wrong::Body) => {
                                counts.bad_body.fetch_add(1, Ordering::Relaxed);
                                counts.note(format!(
                                    "{want} was answered the body {:?}",
                                    String::from_utf8_lossy(&answered.response.body)
                                ));
                            }
                        }
                        if answered.leftover > 0 {
                            counts.leftover.fetch_add(1, Ordering::Relaxed);
                            counts.note(format!(
                                "{} byte(s) arrived after {want}'s answer, which asked once",
                                answered.leftover
                            ));
                        }
                    }
                    Err(err) => {
                        if matches!(err.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) {
                            counts.stalls.fetch_add(1, Ordering::Relaxed);
                        } else {
                            counts.errors.fetch_add(1, Ordering::Relaxed);
                        }
                        held.clear();
                        match open(&target, timeout, false) {
                            Ok(fresh) => sock = fresh,
                            Err(_) => break,
                        }
                    }
                }
            }
            counts.lat_ns.fetch_add(sum, Ordering::Relaxed);
            counts.max_ns.fetch_max(peak, Ordering::Relaxed);
            for (slot, count) in local.iter().enumerate() {
                if *count > 0 {
                    counts.hist[slot].fetch_add(*count, Ordering::Relaxed);
                }
            }
        }));
    }

    ready.wait();
    let started = Instant::now();
    go.wait();
    std::thread::sleep(Duration::from_secs(args.seconds));
    stop.store(true, Ordering::Relaxed);
    let elapsed = started.elapsed().as_secs_f64();
    for thread in threads {
        thread.join().ok();
    }

    let verified = counts.verified.load(Ordering::Relaxed);
    let bad_status = counts.bad_status.load(Ordering::Relaxed);
    let bad_body = counts.bad_body.load(Ordering::Relaxed);
    let leftover = counts.leftover.load(Ordering::Relaxed);
    let answered = verified + bad_status + bad_body;
    let hist: Vec<u64> = counts
        .hist
        .iter()
        .map(|slot| slot.load(Ordering::Relaxed))
        .collect();
    let opened = usize::try_from(opened_count.load(Ordering::Relaxed)).unwrap_or(usize::MAX);
    let opening = opening.lock().expect("the opening lock is never poisoned");
    let mean_us = if answered == 0 {
        0.0
    } else {
        counts.lat_ns.load(Ordering::Relaxed) as f64 / answered as f64 / 1000.0
    };

    let mut record = String::new();
    let _ = write!(
        record,
        "{{\"mode\": \"closed-loop\", \"target\": {target}, {opening_fields}, \
         \"seconds\": {elapsed:.3}, \"requests\": {answered}, \
         \"requests_per_sec\": {rps:.1}, \"verified\": {verified}, \
         \"bad_status\": {bad_status}, \"bad_body\": {bad_body}, \"leftover\": {leftover}, \
         \"stalls\": {stalls}, \"errors\": {errors}, \"mean_us\": {mean_us:.1}, \
         \"p50_us\": {p50}, \"p99_us\": {p99}, \"p999_us\": {p999}, \"max_us\": {max_us:.1}, \
         \"notes\": {notes}}}",
        target = json_string(&args.target),
        opening_fields = opening.fields(args.connections, opened),
        rps = answered as f64 / elapsed,
        stalls = counts.stalls.load(Ordering::Relaxed),
        errors = counts.errors.load(Ordering::Relaxed),
        p50 = percentile(&hist, answered, 50, 100),
        p99 = percentile(&hist, answered, 99, 100),
        p999 = percentile(&hist, answered, 999, 1000),
        max_us = counts.max_ns.load(Ordering::Relaxed) as f64 / 1000.0,
        notes = json_strings(
            &counts
                .notes
                .lock()
                .expect("the notes lock is never poisoned")
        ),
    );
    Outcome {
        record,
        wrong: bad_status + bad_body + leftover,
    }
}

/// One response, and how many bytes came with it that nothing asked for.
#[derive(Debug)]
struct Answered {
    response: Response,
    leftover: usize,
}

/// Writes one request and reads one whole response.
fn exchange(
    sock: &mut TcpStream,
    token: &str,
    buf: &mut [u8],
    held: &mut Vec<u8>,
) -> std::io::Result<Answered> {
    sock.write_all(request_for(token).as_bytes())?;
    loop {
        if let Some(response) = take_response(held) {
            // One request was outstanding, so whatever is still here is a
            // second answer to it.
            let leftover = held.len();
            held.clear();
            return Ok(Answered { response, leftover });
        }
        let read = sock.read(buf)?;
        if read == 0 {
            return Err(std::io::Error::new(ErrorKind::UnexpectedEof, "peer closed"));
        }
        held.extend_from_slice(&buf[..read]);
    }
}

// ---------------------------------------------------------------------------
// In flight: every request out before any answer is read
// ---------------------------------------------------------------------------

/// N requests written before any answer is read, `--rounds` times.
fn in_flight(args: &Args) -> Outcome {
    let run = std::process::id();
    let timeout = Duration::from_millis(args.timeout_ms);
    let mut opening = Opened::default();
    let mut socks: Vec<TcpStream> = Vec::with_capacity(args.connections);
    for conn in 0..args.connections {
        match open(&args.target, timeout, true) {
            Ok(sock) => socks.push(sock),
            Err(err) => {
                opening.refused_at = Some(conn);
                opening.reason = Some(err.to_string());
                break;
            }
        }
    }
    let width = socks.len();
    let mut rounds = Vec::with_capacity(args.rounds);
    let mut notes: Vec<String> = Vec::new();
    let mut wrong = 0u64;

    for round in 1..=args.rounds {
        if width == 0 {
            break;
        }
        let mut held: Vec<Vec<u8>> = vec![Vec::with_capacity(512); width];
        let mut settled = vec![false; width];
        let mut sent = vec![true; width];
        let started = Instant::now();
        // A token per connection *and* per round: an answer held over from the
        // previous round is as wrong as one from another connection.
        let want = |conn: usize| token(run, conn, u64::try_from(round).unwrap_or(u64::MAX));
        for (conn, sock) in socks.iter_mut().enumerate() {
            sent[conn] =
                write_all_nonblocking(sock, request_for(&want(conn)).as_bytes(), timeout).is_ok();
        }
        let out_in = started.elapsed();

        let (mut verified, mut mismatched, mut broken, mut done) = (0usize, 0usize, 0usize, 0usize);
        let deadline = Instant::now() + timeout.max(Duration::from_secs(30));
        while done < width && Instant::now() < deadline {
            let mut moved = false;
            for conn in 0..width {
                if settled[conn] {
                    continue;
                }
                if !sent[conn] {
                    settled[conn] = true;
                    done += 1;
                    broken += 1;
                    continue;
                }
                let mut buf = [0u8; 4096];
                match socks[conn].read(&mut buf) {
                    Ok(0) => {
                        settled[conn] = true;
                        done += 1;
                        broken += 1;
                        moved = true;
                    }
                    Ok(read) => {
                        held[conn].extend_from_slice(&buf[..read]);
                        moved = true;
                        if let Some(response) = take_response(&mut held[conn]) {
                            settled[conn] = true;
                            done += 1;
                            match verify(&want(conn), &response) {
                                Ok(()) => verified += 1,
                                Err(_) => {
                                    mismatched += 1;
                                    if notes.len() < NOTES {
                                        notes.push(format!(
                                            "round {round}: connection {conn} asked {}, was \
                                             answered {} {:?}",
                                            want(conn),
                                            String::from_utf8_lossy(&response.status),
                                            String::from_utf8_lossy(&response.body)
                                        ));
                                    }
                                }
                            }
                        }
                    }
                    Err(ref err) if err.kind() == ErrorKind::WouldBlock => {}
                    Err(_) => {
                        settled[conn] = true;
                        done += 1;
                        broken += 1;
                        moved = true;
                    }
                }
            }
            if !moved {
                std::thread::sleep(Duration::from_micros(200));
            }
        }
        let unanswered = width - done;
        wrong += u64::try_from(mismatched + broken + unanswered).unwrap_or(u64::MAX);
        rounds.push(format!(
            "{{\"round\": {round}, \"in_flight\": {width}, \"verified\": {verified}, \
             \"mismatched\": {mismatched}, \"broken\": {broken}, \"unanswered\": {unanswered}, \
             \"requests_out_in_s\": {out:.3}, \"all_answered_in_s\": {all:.3}}}",
            out = out_in.as_secs_f64(),
            all = started.elapsed().as_secs_f64(),
        ));
    }

    let mut record = String::new();
    let _ = write!(
        record,
        "{{\"mode\": \"in-flight\", \"target\": {target}, {opening_fields}, \
         \"rounds\": [{rounds}], \"notes\": {notes}}}",
        target = json_string(&args.target),
        opening_fields = opening.fields(args.connections, width),
        rounds = rounds.join(", "),
        notes = json_strings(&notes),
    );
    Outcome { record, wrong }
}

/// Writes the whole of `bytes` to a non-blocking socket, or gives up on the
/// deadline rather than spinning for the length of the run.
fn write_all_nonblocking(
    sock: &mut TcpStream,
    mut bytes: &[u8],
    timeout: Duration,
) -> std::io::Result<()> {
    let deadline = Instant::now() + timeout;
    while !bytes.is_empty() {
        match sock.write(bytes) {
            Ok(0) => return Err(std::io::Error::new(ErrorKind::WriteZero, "peer closed")),
            Ok(wrote) => bytes = &bytes[wrote..],
            Err(ref err) if err.kind() == ErrorKind::WouldBlock => {
                if Instant::now() > deadline {
                    return Err(std::io::Error::new(ErrorKind::TimedOut, "write"));
                }
                std::thread::sleep(Duration::from_micros(100));
            }
            Err(err) => return Err(err),
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// The record's arithmetic and quoting
// ---------------------------------------------------------------------------

/// Which power-of-two microsecond bucket a duration falls in.
fn bucket(ns: u64) -> usize {
    let us = (ns / 1000).max(1);
    let leading = usize::try_from(us.leading_zeros()).unwrap_or(0);
    (64 - leading - 1).min(BUCKETS - 1)
}

/// The upper edge of the bucket the quantile `numerator/denominator` falls in.
///
/// A power-of-two bound rather than an interpolation, so the figure is never
/// finer than the histogram behind it — a `p99_us` of 512 means "at or under
/// 512", and reading it as 512 exactly is reading a bucket as a measurement.
///
/// The quantile is a pair of integers rather than a float because the rank it
/// picks is exact arithmetic on a count: through an `f64` a long run's rank
/// rounds, and the boundary between two buckets is where that shows up.
fn percentile(hist: &[u64], total: u64, numerator: u64, denominator: u64) -> u64 {
    if total == 0 {
        return 0;
    }
    let want = (total * numerator).div_ceil(denominator);
    let mut seen = 0u64;
    for (slot, count) in hist.iter().enumerate() {
        seen += count;
        if seen >= want {
            return 1u64 << (slot + 1);
        }
    }
    0
}

/// One JSON string, with the characters a body or an error message can carry.
fn json_string(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if (ch as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", ch as u32);
            }
            ch => out.push(ch),
        }
    }
    out.push('"');
    out
}

/// A JSON array of strings.
fn json_strings(lines: &[String]) -> String {
    let items: Vec<String> = lines.iter().map(|line| json_string(line)).collect();
    format!("[{}]", items.join(", "))
}

#[cfg(test)]
mod tests {
    use super::{Response, Wrong, body_for, bucket, percentile, take_response, token, verify};

    /// The check this whole leg rests on: an answer naming a different request
    /// is not this request's answer, however well-formed it is.
    #[test]
    fn a_body_naming_another_request_is_rejected() {
        let mine = token(1, 2, 3);
        let theirs = token(1, 9, 3);
        let answer = Response {
            status: *b"200",
            body: body_for(&theirs).into_bytes(),
        };
        assert_eq!(verify(&mine, &answer), Err(Wrong::Body));
        assert_eq!(verify(&theirs, &answer), Ok(()));
    }

    /// The same connection, one request behind: the skew a constant body hides
    /// for the whole of a run.
    #[test]
    fn an_answer_to_this_connections_previous_request_is_rejected() {
        let now = token(1, 2, 8);
        let before = token(1, 2, 7);
        let answer = Response {
            status: *b"200",
            body: body_for(&before).into_bytes(),
        };
        assert_eq!(verify(&now, &answer), Err(Wrong::Body));
    }

    /// **The guard on the guard.** Point this probe at the constant-response
    /// case every other serve leg uses and every request has to fail, because a
    /// verifier that passes `hello, world` has stopped verifying and a run that
    /// went green would mean nothing.
    #[test]
    fn a_constant_body_fails_every_request() {
        let answer = Response {
            status: *b"200",
            body: b"hello, world\n".to_vec(),
        };
        for seq in 0..64 {
            assert_eq!(verify(&token(7, 1, seq), &answer), Err(Wrong::Body));
        }
    }

    #[test]
    fn a_status_that_is_not_200_is_rejected_before_the_body_is_read() {
        let mine = token(1, 2, 3);
        let answer = Response {
            status: *b"503",
            body: body_for(&mine).into_bytes(),
        };
        assert_eq!(verify(&mine, &answer), Err(Wrong::Status));
    }

    #[test]
    fn a_response_is_framed_off_by_its_content_length() {
        let mut held = b"HTTP/1.1 200 OK\r\ncontent-length: 5\r\n\r\nhello".to_vec();
        let response = take_response(&mut held).expect("a whole response");
        assert_eq!(response.status, *b"200");
        assert_eq!(response.body, b"hello");
        assert!(held.is_empty(), "the framed response was not consumed");
    }

    #[test]
    fn a_response_that_has_not_all_arrived_is_not_framed_yet() {
        let mut held = b"HTTP/1.1 200 OK\r\ncontent-length: 5\r\n\r\nhel".to_vec();
        assert!(take_response(&mut held).is_none());
        held.extend_from_slice(b"lo");
        assert!(take_response(&mut held).is_some());
    }

    /// A second answer to a request that asked once stays in the buffer, which
    /// is what the caller counts as `leftover`.
    #[test]
    fn bytes_past_one_response_are_left_for_the_caller_to_count() {
        let mut held =
            b"HTTP/1.1 200 OK\r\ncontent-length: 2\r\n\r\nhiHTTP/1.1 200 OK\r\ncontent-length: 2\r\n\r\nhi"
                .to_vec();
        take_response(&mut held).expect("the first response");
        assert!(!held.is_empty(), "the second answer was swallowed");
    }

    #[test]
    fn a_percentile_is_the_upper_edge_of_its_bucket() {
        let mut hist = vec![0u64; super::BUCKETS];
        hist[bucket(20_000)] = 99; // 20us
        hist[bucket(9_000_000)] = 1; // 9ms
        assert_eq!(percentile(&hist, 100, 50, 100), 32);
        assert_eq!(percentile(&hist, 100, 999, 1000), 16_384);
    }

    #[test]
    fn a_token_names_one_run_one_connection_and_one_position() {
        assert_ne!(token(1, 2, 3), token(2, 2, 3));
        assert_ne!(token(1, 2, 3), token(1, 3, 3));
        assert_ne!(token(1, 2, 3), token(1, 2, 4));
    }
}
