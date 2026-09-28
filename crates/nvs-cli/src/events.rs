//! `nvs run --events <file>` — an event-stream connection run with no server
//! in front of it, and a publisher read off a file.
//!
//! [`crate::peer`]'s twin for the other door
//! (`rule:concurrency/two-doors-one-isolate`). `Core\Sse::current` and
//! `Core\Sse::receive` ask their context whether it is the isolate
//! `Core\Sse::upgrade` opens: a context holding the writing half of a
//! `text/event-stream` body and marked
//! [`nvs_runtime::EventStreamDoor::Connection`]. Only
//! `nvs_host::Isolate::over_event_stream` answers yes inside a server. This
//! module is the second thing that does, so an example, an attack or a bench of
//! those members has something to run against.
//!
//! # The format
//!
//! One line per value, read top to bottom. A blank line and a line starting
//! `#` are skipped.
//!
//! ```text
//! publish: room:feed The board changed    the value `The board changed`, on the topic `room:feed`
//! ```
//!
//! The topic is the first word after `publish:` and the value is the rest of
//! the line, so a topic cannot hold a space. A value is published as a
//! `string`.
//!
//! # When a value is published, and when the stream ends
//!
//! A value is published through [`nvs_stdlib::publish_text`], the bus a
//! program's own `Core\Topic::publish` uses, and **only while the program
//! waits in `receive` with nothing queued**. That is the one moment every
//! subscription it will make before that wait already exists, so a run is the
//! same every time, and a value on a topic the program never subscribed to
//! reaches nobody, as it would in a server. Once the file is used up, the
//! next such wait ends the stream: the connection's half is dropped as a
//! client that went away drops it, and `receive` answers `null`.
//!
//! # What goes to standard output
//!
//! The body exactly as a client reads it — `event:`, `data:` and `retry:`
//! lines and the blank line after each event — written through
//! [`std::io::stdout`], the handle `echo` writes through, so an example's
//! `.out` reads as the stream with the program's own output in the order the
//! two happened.
//!
//! **What it spends:** the file's values, held for the run, and one chunk of
//! the stream at a time. The bounds are `nvs_server::bounds::Connection`'s
//! defaults, the ones a served stream is opened under before configuration
//! changes them.

use std::cell::Cell;
use std::collections::VecDeque;
use std::io::Write;
use std::rc::Rc;
use std::time::Duration;

use nvs_runtime::stream::{Drain, Drained, Emit};

/// How long the publisher waits between two looks at the program. Short,
/// because the program's own wait is one tick of `Core\Sse::receive` and the
/// publisher's look is what ends it.
const TURN: Duration = Duration::from_millis(1);

/// One `publish:` line: the topic, then the value.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Publish {
    topic: String,
    value: String,
}

/// The values `nvs run --events <file>` publishes, in the file's order.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Feed {
    values: VecDeque<Publish>,
}

/// Reads the file `nvs run --events <file>` names.
///
/// # Errors
///
/// The file could not be read, or [`read`] refused its text.
pub(crate) fn from_file(path: &std::path::Path) -> Result<Feed, String> {
    let text = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
    read(&text)
}

/// Parses the module doc's format.
///
/// # Errors
///
/// A line that is not a `publish:` line, and a `publish:` line with no topic,
/// each named by its line number.
pub(crate) fn read(text: &str) -> Result<Feed, String> {
    let mut values = VecDeque::new();
    for (index, line) in text.lines().enumerate() {
        let number = index + 1;
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let Some(rest) = line.strip_prefix("publish:") else {
            return Err(format!(
                "line {number}: expected `publish: <topic> <value>`"
            ));
        };
        let rest = rest.trim_start();
        let (topic, value) = rest.split_once(' ').unwrap_or((rest, ""));
        if topic.is_empty() {
            return Err(format!("line {number}: `publish:` needs a topic"));
        }
        values.push_back(Publish {
            topic: topic.to_owned(),
            value: value.to_owned(),
        });
    }
    Ok(Feed { values })
}

/// The two halves of the body a served stream would be handed, at the bounds
/// a served stream is opened under.
pub(crate) fn open() -> (Emit, Drain) {
    let bounds = nvs_server::bounds::Connection::default();
    nvs_runtime::stream::open(bounds.send, bounds.message)
}

/// Spawns the publisher onto `sched`, beside the program's own task.
///
/// `inbox` is the program's delivery queue, taken off its context before the
/// context moved into its task, and `ended` is the cell that task writes its
/// status into, which is how the publisher knows to stop: a task polling on a
/// timer is always runnable, so `nvs_host::run_until_idle` would never return
/// while it ran.
pub(crate) fn start(
    sched: &mut nvs_host::Scheduler,
    feed: Feed,
    drain: Drain,
    inbox: Rc<nvs_runtime::Inbox>,
    ended: Rc<Cell<Option<Result<(), i32>>>>,
) {
    sched.spawn(
        nvs_runtime::Ctx::stdout(),
        nvs_runtime::TaskRoot::Worker,
        move |_| publish(feed, drain, &inbox, &ended),
    );
}

/// The publisher's whole task: print what the program wrote, and feed it the
/// next value each time it waits for one.
fn publish(
    mut feed: Feed,
    drain: Drain,
    inbox: &nvs_runtime::Inbox,
    ended: &Cell<Option<Result<(), i32>>>,
) {
    let mut drain = Some(drain);
    loop {
        if let Some(drain) = drain.as_mut() {
            print_written(drain);
        }
        if ended.get().is_some() {
            return;
        }
        if inbox.is_empty() && inbox.is_waiting() {
            match feed.values.pop_front() {
                Some(next) => {
                    let _reached = nvs_stdlib::publish_text(&next.topic, &next.value);
                }
                // The client going away, which is what `receive` reads as the
                // end of the stream.
                None => drop(drain.take()),
            }
        }
        match nvs_runtime::host::with_current(|host| host.sleep(TURN)) {
            Some(nvs_runtime::host::Woken::Elapsed) => {}
            Some(nvs_runtime::host::Woken::Cancelled) | None => return,
        }
    }
}

/// Writes every chunk the program has handed over so far to standard output.
///
/// A failed write is not the program's to see, for the reason `echo`'s is
/// not: the run's own output is closed.
fn print_written(drain: &mut Drain) {
    let mut out = std::io::stdout();
    while let Drained::Chunk(chunk) = drain.next_chunk(std::task::Waker::noop()) {
        let _ = out.write_all(&chunk);
    }
    let _ = out.flush();
}

#[cfg(test)]
mod tests {
    use super::{Publish, read};

    /// Every line is one value on one topic, in the file's order, and the
    /// value keeps its own spacing.
    #[test]
    fn an_events_file_lists_its_values_in_order() {
        let feed = read("# a comment\n\npublish: room:feed Hello,  world\npublish: news\n")
            .expect("the file is well formed");
        assert_eq!(
            feed.values.into_iter().collect::<Vec<_>>(),
            vec![
                Publish {
                    topic: "room:feed".to_owned(),
                    value: "Hello,  world".to_owned(),
                },
                Publish {
                    topic: "news".to_owned(),
                    value: String::new(),
                },
            ]
        );
    }

    /// A line that is not a `publish:` line is refused with its number, and so
    /// is one with no topic.
    #[test]
    fn a_malformed_events_file_is_refused_by_line() {
        for (text, needle) in [
            ("publish: a b\nhello\n", "line 2"),
            ("publish:\n", "needs a topic"),
            ("text: hi\n", "expected `publish:"),
        ] {
            let refused = read(text).expect_err(text);
            assert!(
                refused.contains(needle),
                "{text:?} was refused with {refused:?}"
            );
        }
    }
}
