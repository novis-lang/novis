//! What a client reads while the program is stopped: each stopped task as a thread, its frames and
//! their variables (`rule:testing/a-stop-pauses-the-whole-task-tree`).
//!
//! The controller sends one [`Stop`] per task, and every task that one breakpoint or one pause
//! stops has the same round. The first stop of a round is the `stopped` event, with every thread
//! stopped, and the round's threads are its tasks in the order their stops arrived.
//!
//! A stop the session reads after it resumed that stop's round is dropped: the resume woke its
//! task, which runs again. A task of the tree can send its stop just before a `next` and the
//! session read it just after, and filed as a new round it would be a second `stopped` event, a
//! `pause` the client never asked for.
//!
//! After a step left a function, the innermost frame's scope starts with the value that function
//! returned, named `f() returned`.
//!
//! A stop for an exception names its class in the `stopped` event's `description`, as `Uncaught`
//! or `Thrown` and the class, with its message as the `text`. `exceptionInfo` gives the same.
//!
//! **Ids.** A thread id is the task's place in the round, from 1. A frame id is its thread and its
//! place on that thread's stack. A variables reference is an index into the round's references,
//! each a frame's scope or a value's children. Every id belongs to one round, and the resume that
//! ends the round forgets them all.
//!
//! **What it spends:** the round's stops, and one reference per scope or value the client
//! expanded, the value's children copied into it. All of it is freed at the resume.

use nvs_host::debug::{Stop, StopReason, StoppedFrame};
use nvs_render::{Node, Scalar};
use serde_json::{Value, json};

use super::paths::PathMap;

/// How many frames of one thread a frame id has room for.
const FRAMES: i64 = 1 << 20;

/// The stop the client is reading, if the program is stopped.
#[derive(Default)]
pub(super) struct Stopped {
    round: Option<Round>,
    /// How many resumes the session made, which is the round of the next stop. A stop of an
    /// earlier round arrived after its resume.
    resumes: u64,
    /// The launch's mappings, which a frame's file is shown through.
    pub(super) paths: PathMap,
}

struct Round {
    number: u64,
    stops: Vec<Stop>,
    refs: Vec<Ref>,
}

/// What one variables reference shows.
enum Ref {
    /// The variables of one frame.
    Frame { thread: usize, frame: usize },
    /// The entries of one array, or the properties of one object.
    Children(Vec<(String, Node)>),
}

impl Stopped {
    /// Files one task's stop. Gives the `stopped` event's body for the first stop of a round, and
    /// `None` for each stop after it and for a stop of a round already resumed.
    pub(super) fn file(&mut self, stop: Stop) -> Option<Value> {
        if stop.round < self.resumes {
            return None;
        }
        if let Some(round) = &mut self.round
            && round.number == stop.round
        {
            round.stops.push(stop);
            return None;
        }
        let reason = match stop.reason {
            StopReason::Breakpoint => "breakpoint",
            StopReason::Pause | StopReason::Tree => "pause",
            StopReason::Entry => "entry",
            StopReason::Step => "step",
            StopReason::Exception => "exception",
        };
        let mut event = json!({ "reason": reason, "threadId": 1, "allThreadsStopped": true });
        if let Some(exception) = &stop.exception {
            let kind = if exception.uncaught {
                "Uncaught"
            } else {
                "Thrown"
            };
            event["description"] = json!(format!("{kind} {}", exception.class));
            event["text"] = json!(exception.message);
        }
        self.round = Some(Round {
            number: stop.round,
            stops: vec![stop],
            refs: Vec::new(),
        });
        Some(event)
    }

    /// The `exceptionInfo` body for the thread `arguments` names, which stopped for an exception.
    pub(super) fn exception_info(&self, arguments: &Value) -> Result<Value, String> {
        let round = self.round()?;
        let exception = arguments["threadId"]
            .as_i64()
            .and_then(|id| usize::try_from(id - 1).ok())
            .and_then(|thread| round.stops.get(thread))
            .and_then(|stop| stop.exception.as_ref())
            .ok_or("This thread did not stop for an exception.")?;
        let mode = if exception.uncaught {
            "unhandled"
        } else {
            "always"
        };
        Ok(json!({
            "exceptionId": exception.class,
            "description": exception.message,
            "breakMode": mode,
        }))
    }

    /// Forgets the round, and counts the resume that follows. Called before each resume, so a stop
    /// of the next round is never cleared.
    pub(super) fn clear(&mut self) {
        self.round = None;
        self.resumes += 1;
    }

    /// The `threads` body: the round's tasks, or the one `main` thread while nothing is stopped.
    pub(super) fn threads(&self) -> Value {
        let threads: Vec<Value> = match &self.round {
            None => vec![json!({ "id": 1, "name": "main" })],
            Some(round) => (1_i64..)
                .zip(&round.stops)
                .map(|(id, stop)| json!({ "id": id, "name": thread_name(stop) }))
                .collect(),
        };
        json!({ "threads": threads })
    }

    /// The stop of the thread `arguments` names, as [`nvs_host::debug::Debugger::step`] names it.
    pub(super) fn stop_id(&self, arguments: &Value) -> Result<u64, String> {
        let round = self.round()?;
        arguments["threadId"]
            .as_i64()
            .and_then(|id| usize::try_from(id - 1).ok())
            .and_then(|thread| round.stops.get(thread))
            .map(|stop| stop.id)
            .ok_or_else(|| "No stopped thread has this id.".to_owned())
    }

    /// The `stackTrace` body for the thread `arguments` names, innermost frame first.
    pub(super) fn stack_trace(&self, arguments: &Value) -> Result<Value, String> {
        let round = self.round()?;
        let thread = arguments["threadId"]
            .as_i64()
            .and_then(|id| usize::try_from(id - 1).ok())
            .filter(|&thread| thread < round.stops.len())
            .ok_or("No stopped thread has this id.")?;
        let frames = &round.stops[thread].frames;
        let start = number(&arguments["startFrame"]).unwrap_or(0);
        let levels = number(&arguments["levels"])
            .filter(|&levels| levels > 0)
            .unwrap_or(usize::MAX);
        let shown: Vec<Value> = frames
            .iter()
            .enumerate()
            .skip(start)
            .take(levels)
            .map(|(index, frame)| stack_frame(frame_id(thread, index), frame, &self.paths))
            .collect();
        Ok(json!({ "stackFrames": shown, "totalFrames": frames.len() }))
    }

    /// The `scopes` body for the frame `arguments` names: one scope with its local variables.
    pub(super) fn scopes(&mut self, arguments: &Value) -> Result<Value, String> {
        let round = self.round_mut()?;
        let (thread, frame) = arguments["frameId"]
            .as_i64()
            .and_then(|id| {
                let thread = usize::try_from((id - 1) / FRAMES).ok()?;
                let frame = usize::try_from((id - 1) % FRAMES).ok()?;
                round.stops.get(thread)?.frames.get(frame)?;
                Some((thread, frame))
            })
            .ok_or("No stopped frame has this id.")?;
        round.refs.push(Ref::Frame { thread, frame });
        Ok(json!({ "scopes": [{
            "name": "Locals",
            "presentationHint": "locals",
            "variablesReference": round.refs.len(),
            "expensive": false,
        }] }))
    }

    /// The `variables` body for the reference `arguments` names.
    pub(super) fn variables(&mut self, arguments: &Value) -> Result<Value, String> {
        let round = self.round_mut()?;
        let entries = arguments["variablesReference"]
            .as_i64()
            .and_then(|id| usize::try_from(id - 1).ok())
            .and_then(|index| round.refs.get(index))
            .map(|shown| match shown {
                Ref::Frame { thread, frame } => {
                    let stop = &round.stops[*thread];
                    let returned = stop
                        .returned
                        .as_ref()
                        .filter(|_| *frame == 0)
                        .map(|returned| {
                            (
                                format!("{}() returned", returned.function),
                                returned.value.clone(),
                            )
                        });
                    returned
                        .into_iter()
                        .chain(stop.frames[*frame].variables.iter().map(|variable| {
                            (format!("${}", variable.name), variable.value.clone())
                        }))
                        .collect()
                }
                Ref::Children(children) => children.clone(),
            })
            .ok_or("No value has this reference.")?;
        let variables: Vec<Value> = entries
            .into_iter()
            .map(|(name, node)| {
                let reference = match children(&node) {
                    Some(children) => {
                        round.refs.push(Ref::Children(children));
                        round.refs.len()
                    }
                    None => 0,
                };
                let mut variable = json!({
                    "name": name,
                    "value": summary(&node),
                    "variablesReference": reference,
                });
                if let Some(kind) = type_name(&node) {
                    variable["type"] = json!(kind);
                }
                variable
            })
            .collect();
        Ok(json!({ "variables": variables }))
    }

    fn round(&self) -> Result<&Round, String> {
        self.round
            .as_ref()
            .ok_or_else(|| "The program is not stopped.".to_owned())
    }

    fn round_mut(&mut self) -> Result<&mut Round, String> {
        self.round
            .as_mut()
            .ok_or_else(|| "The program is not stopped.".to_owned())
    }
}

/// A thread's name: its task, or `main` for a context with no scheduler beneath it.
fn thread_name(stop: &Stop) -> String {
    stop.task
        .map_or_else(|| "main".to_owned(), |task| format!("task {}", task.raw()))
}

fn frame_id(thread: usize, index: usize) -> i64 {
    let thread = i64::try_from(thread).unwrap_or(i64::MAX / FRAMES - 1);
    let index = i64::try_from(index).unwrap_or(FRAMES - 1).min(FRAMES - 1);
    thread * FRAMES + index + 1
}

fn number(value: &Value) -> Option<usize> {
    value.as_u64().and_then(|n| usize::try_from(n).ok())
}

/// One frame as DAP shows it, its file mapped to the editor's path. A frame with no known site
/// has line 0 and no source.
fn stack_frame(id: i64, frame: &StoppedFrame, paths: &PathMap) -> Value {
    let name = if frame.name.is_empty() {
        "(unknown)"
    } else {
        &frame.name
    };
    match &frame.at {
        Some(at) => {
            let path = paths.to_editor(&at.file);
            let file_name = std::path::Path::new(&path)
                .file_name()
                .map_or_else(|| path.clone(), |name| name.to_string_lossy().into_owned());
            json!({
                "id": id,
                "name": name,
                "line": at.line,
                "column": 1,
                "source": { "name": file_name, "path": path },
            })
        }
        None => json!({
            "id": id,
            "name": name,
            "line": 0,
            "column": 0,
            "presentationHint": "subtle",
        }),
    }
}

/// The entries a client may expand under a value, or `None` for a value with none.
fn children(node: &Node) -> Option<Vec<(String, Node)>> {
    let children: Vec<(String, Node)> = match node {
        Node::Sequence(items) => items
            .iter()
            .enumerate()
            .map(|(index, item)| (index.to_string(), item.clone()))
            .collect(),
        Node::Map(entries) => entries
            .iter()
            .map(|(key, value)| (key.as_str().to_owned(), value.clone()))
            .collect(),
        Node::Object { properties, .. } => properties.clone(),
        _ => return None,
    };
    (!children.is_empty()).then_some(children)
}

/// The one line a value shows beside its name.
pub(super) fn summary(node: &Node) -> String {
    match node {
        Node::Scalar(Scalar::Null) => "null".to_owned(),
        Node::Scalar(Scalar::Bool(value)) => value.to_string(),
        Node::Scalar(Scalar::Int(value)) => value.to_string(),
        Node::Scalar(Scalar::Uint(value)) => value.to_string(),
        Node::Scalar(Scalar::Float(value)) => format!("{value:?}"),
        Node::Scalar(Scalar::Decimal(value)) => value.clone(),
        Node::Scalar(Scalar::Str { text, .. }) => format!("{:?}", text.as_str()),
        Node::Sequence(items) => format!("array({})", items.len()),
        Node::Map(entries) => format!("array({})", entries.len()),
        Node::Object { class, .. } => class.clone(),
        Node::EnumCase { enum_name, case } => format!("{enum_name}::{case}"),
        other => nvs_render::plain::render_nodes(std::slice::from_ref(other))
            .trim_end()
            .to_owned(),
    }
}

/// The type a value shows, where it has one.
fn type_name(node: &Node) -> Option<String> {
    let name = match node {
        Node::Scalar(Scalar::Null) => "null",
        Node::Scalar(Scalar::Bool(_)) => "bool",
        Node::Scalar(Scalar::Int(_)) => "int",
        Node::Scalar(Scalar::Uint(_)) => "uint",
        Node::Scalar(Scalar::Float(_)) => "float",
        Node::Scalar(Scalar::Decimal(_)) => "decimal",
        Node::Scalar(Scalar::Str { .. }) => "string",
        Node::Scalar(Scalar::Bytes(_)) => "bytes",
        Node::Sequence(_) | Node::Map(_) => "array",
        Node::Object { class, .. } => return Some(class.clone()),
        Node::EnumCase { enum_name, .. } => return Some(enum_name.clone()),
        _ => return None,
    };
    Some(name.to_owned())
}

#[cfg(test)]
mod tests {
    use nvs_host::debug::{SiteLine, StoppedVariable};

    use super::*;

    fn stop(round: u64, reason: StopReason) -> Stop {
        Stop {
            id: round,
            task: None,
            reason,
            round,
            returned: None,
            exception: None,
            frames: vec![StoppedFrame {
                function: 0,
                name: "<script>".to_owned(),
                site: 0,
                at: Some(SiteLine {
                    file: "a.nvs".to_owned(),
                    line: 3,
                }),
                variables: vec![StoppedVariable {
                    name: "items".to_owned(),
                    value: Node::Sequence(vec![Node::Scalar(Scalar::Int(7))]),
                }],
            }],
        }
    }

    /// The first stop of a round is the event, a later stop of the same round joins it as a
    /// second thread, and a new round starts again from one thread.
    #[test]
    fn a_round_is_one_event_and_each_of_its_stops_a_thread() {
        let mut stopped = Stopped::default();
        let event = stopped.file(stop(0, StopReason::Breakpoint));
        assert_eq!(event.unwrap()["reason"], "breakpoint");
        assert!(stopped.file(stop(0, StopReason::Tree)).is_none());
        assert_eq!(stopped.threads()["threads"].as_array().unwrap().len(), 2);

        stopped.clear();
        assert_eq!(stopped.threads()["threads"][0]["name"], "main");
        assert_eq!(
            stopped.file(stop(1, StopReason::Pause)).unwrap()["reason"],
            "pause"
        );
        assert_eq!(stopped.threads()["threads"].as_array().unwrap().len(), 1);
    }

    /// A stop of a round the session already resumed is no event and no thread, and the next
    /// round's first stop is still the event.
    #[test]
    fn a_stop_read_after_its_round_was_resumed_is_dropped() {
        let mut stopped = Stopped::default();
        assert!(stopped.file(stop(0, StopReason::Breakpoint)).is_some());
        stopped.clear();
        assert!(stopped.file(stop(0, StopReason::Tree)).is_none());
        assert_eq!(stopped.threads()["threads"][0]["name"], "main");
        assert_eq!(
            stopped.file(stop(1, StopReason::Step)).unwrap()["reason"],
            "step"
        );
        assert_eq!(stopped.threads()["threads"].as_array().unwrap().len(), 1);
    }

    /// A frame's scope lists its variables, and an array expands into its entries.
    #[test]
    fn a_frame_scope_shows_its_variables_and_an_array_expands() {
        let mut stopped = Stopped::default();
        stopped.file(stop(0, StopReason::Breakpoint));
        let trace = stopped.stack_trace(&json!({ "threadId": 1 })).unwrap();
        let frame = &trace["stackFrames"][0];
        assert_eq!(
            (frame["line"].clone(), frame["source"]["name"].clone()),
            (json!(3), json!("a.nvs"))
        );

        let scopes = stopped.scopes(&json!({ "frameId": frame["id"] })).unwrap();
        let reference = scopes["scopes"][0]["variablesReference"].clone();
        let locals = stopped
            .variables(&json!({ "variablesReference": reference }))
            .unwrap();
        let items = &locals["variables"][0];
        assert_eq!(
            (items["name"].clone(), items["value"].clone()),
            (json!("$items"), json!("array(1)"))
        );

        let entries = stopped
            .variables(&json!({ "variablesReference": items["variablesReference"] }))
            .unwrap();
        assert_eq!(entries["variables"][0]["value"], "7");
        assert_eq!(entries["variables"][0]["type"], "int");
    }

    /// After a step left a function, the innermost frame's scope shows what it returned first,
    /// and an outer frame's scope does not show it.
    #[test]
    fn a_step_out_shows_the_returned_value_first_in_the_innermost_scope() {
        let mut stopped = Stopped::default();
        let mut step = stop(0, StopReason::Step);
        step.returned = Some(nvs_host::debug::StoppedReturn {
            function: "total".to_owned(),
            value: Node::Scalar(Scalar::Int(12)),
        });
        let outer = step.frames[0].clone();
        step.frames.push(outer);
        assert_eq!(stopped.file(step).unwrap()["reason"], "step");
        let locals = |stopped: &mut Stopped, frame: usize| {
            let trace = stopped.stack_trace(&json!({ "threadId": 1 })).unwrap();
            let id = trace["stackFrames"][frame]["id"].clone();
            let scopes = stopped.scopes(&json!({ "frameId": id })).unwrap();
            let reference = scopes["scopes"][0]["variablesReference"].clone();
            stopped
                .variables(&json!({ "variablesReference": reference }))
                .unwrap()["variables"]
                .clone()
        };
        let inner = locals(&mut stopped, 0);
        assert_eq!(
            (inner[0]["name"].clone(), inner[0]["value"].clone()),
            (json!("total() returned"), json!("12"))
        );
        assert_eq!(inner[1]["name"], "$items");
        assert_eq!(locals(&mut stopped, 1)[0]["name"], "$items");
    }

    /// Nothing can be read while the program runs.
    #[test]
    fn a_running_program_has_no_frames() {
        let stopped = Stopped::default();
        assert!(stopped.stack_trace(&json!({ "threadId": 1 })).is_err());
    }
}
