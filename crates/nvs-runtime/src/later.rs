//! `rule:core-classes/html-later`'s slots on a normal route: what
//! `Core\Html::later` registered, the placeholder each one wrote, and the one
//! pass that replaces every placeholder with its output.
//!
//! # A slot is a registration, and the frame ending is what runs it
//!
//! `Core\Html::later` records its closure here and returns the placeholder. The
//! closures run when the request's own frame has returned, all of them at once
//! as one group through [`crate::host::Host::run_group`], and the response is
//! the body with each placeholder replaced by its output. The rule's "starts
//! `fn` at once" is therefore met for the slots among themselves and not yet
//! against the main script: a slot's work does not overlap the frame that
//! registered it. What closing that needs is a host call that starts a child
//! and returns, which the seam does not have; the goal's handoff carries it.
//!
//! **Each slot is a group of one inside the outer group**, so that one slot
//! failing does not cancel its siblings: the outer group only ever sees a slot
//! end with its bytes. A throw or the slot's own `deadline` ends the inner
//! group and the slot answers its `error` fragment. A `FATAL` — a limit of the
//! request tree, which every slot shares (`rule:security/isolate-budget-is-the-trees`)
//! — is recorded on the slot's context, so the outer group stops every sibling
//! and the request fails as it would have without `later`.
//!
//! **A slot cannot change the response head.** `fill` sets
//! [`Ctx::in_later_slot`] on the slot's context, [`Ctx::child`] copies it into
//! every task the slot starts, and each `Core` member that sets a status, a
//! header, a cookie or a body, or regenerates the session, throws `LogicError`
//! when it reads it.
//!
//! **A nested `later` is filled in the same pass.** A slot's context registers
//! its own slots, and the slot assembles its own output before it answers, so
//! the outer pass only ever sees finished bytes.
//!
//! # The placeholder
//!
//! `<?start name="nvs-<token>-<n>">…<?end>`, where `<token>` is 96 random bits
//! drawn once per request and written as 16 URL-safe base64 characters. A
//! visitor's string is escaped before it reaches the sink, so it cannot write
//! `<?`, and an author cannot know the token. A placeholder found nowhere in the
//! body cancels its slot before it starts, with a `Warn`; one found twice fails
//! the request with `LogicError`, because only one copy could be replaced.
//!
//! # What it spends
//!
//! Nothing for a request that never calls `later`: one `Option<Box<_>>` word.
//! A request that does holds one entry per slot — the closure reference, its
//! two fragments and the placeholder bytes — until the pass, and each slot's
//! output until it is spliced. All of it is the request's, under its memory
//! cap, and freed with it.

use std::time::Duration;

use nvs_render::Level;

use crate::ctx::Ctx;
use crate::host::{Bounds, Job, Outcome};
use crate::string::NvsStr;
use crate::throwable::ThrownClass;
use crate::value::Value;

/// The URL-safe base64 alphabet a token is written in.
const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

/// One request's slots: the token every placeholder carries, the number the
/// next one takes, and the slots not yet run.
#[derive(Debug)]
pub struct Slots {
    token: [u8; 16],
    next: u32,
    entries: Vec<Slot>,
}

/// One `Core\Html::later` call that has not run yet.
#[derive(Debug)]
struct Slot {
    /// Owned: the registration outlives the call that made it.
    closure: Value,
    /// The whole placeholder, as it was handed to the page.
    marker: Vec<u8>,
    /// What the slot shows when its closure throws or passes its deadline.
    error: Vec<u8>,
    deadline: Option<Duration>,
}

impl Slots {
    fn drawn() -> Self {
        use rand::Rng;
        let mut raw = [0u8; 12];
        rand::rng().fill_bytes(&mut raw);
        let mut token = [0u8; 16];
        for (chunk, out) in raw.chunks(3).zip(token.chunks_mut(4)) {
            let bits = u32::from(chunk[0]) << 16 | u32::from(chunk[1]) << 8 | u32::from(chunk[2]);
            for (index, byte) in out.iter_mut().enumerate() {
                *byte = ALPHABET[((bits >> (18 - 6 * index)) & 63) as usize];
            }
        }
        Self {
            token,
            next: 0,
            entries: Vec::new(),
        }
    }

    /// A child's slots: the same token, so a nested placeholder is one only
    /// `later` could have written, and nothing registered yet.
    pub(crate) fn sharing_token(&self) -> Self {
        Self {
            token: self.token,
            next: 0,
            entries: Vec::new(),
        }
    }
}

impl Drop for Slots {
    fn drop(&mut self) {
        for slot in self.entries.drain(..) {
            #[expect(
                unsafe_code,
                reason = "the registration owns exactly one reference to its \
                          closure, and a slot dropped unrun is where it is given back"
            )]
            // SAFETY: the reference was retained for this entry and nothing
            // else points at it.
            unsafe {
                slot.closure.release();
            }
        }
    }
}

impl Ctx {
    /// Registers `closure` as a slot and answers the placeholder bytes the page
    /// writes where its output belongs.
    ///
    /// The caller passes an **owned** reference, which the slot keeps until it
    /// runs or the request ends.
    pub fn register_later(
        &mut self,
        closure: Value,
        placeholder: &[u8],
        error: &[u8],
        deadline: Option<Duration>,
    ) -> Vec<u8> {
        let slots = self.later.get_or_insert_with(|| Box::new(Slots::drawn()));
        let number = slots.next;
        slots.next += 1;
        let mut marker = Vec::with_capacity(placeholder.len() + 48);
        marker.extend_from_slice(b"<?start name=\"nvs-");
        marker.extend_from_slice(&slots.token);
        marker.extend_from_slice(format!("-{number}\">").as_bytes());
        marker.extend_from_slice(placeholder);
        marker.extend_from_slice(b"<?end>");
        slots.entries.push(Slot {
            closure,
            marker: marker.clone(),
            error: error.to_vec(),
            deadline,
        });
        marker
    }

    /// Whether this context has slots waiting for the pass.
    #[must_use]
    pub fn has_later(&self) -> bool {
        self.later
            .as_ref()
            .is_some_and(|slots| !slots.entries.is_empty())
    }

    /// Whether this context runs a slot's closure, or a task inside one. Every
    /// `Core` member that changes the response head reads it and throws
    /// `LogicError` when it is set.
    #[must_use]
    pub fn in_later_slot(&self) -> bool {
        self.in_later
    }
}

/// Runs every slot this request registered and replaces each placeholder in
/// the body with its output.
///
/// Called by whoever ran the request's own frame, right after it returned
/// ordinarily and before the answer is taken — `nvs_host::isolate`'s two
/// completion paths. A request with no slot pays one branch, and a request that
/// threw, exited or failed runs none of them: its slots go down with it.
pub fn run_later(ctx: &mut Ctx) {
    if !ctx.has_later() || ctx.pending().is_some() || ctx.ending().is_err() {
        return;
    }
    let Some(mut body) = ctx.take_buffered_output() else {
        return;
    };
    assemble(ctx, &mut body);
    ctx.restore_body(body);
}

/// The pass itself, over `body`, for the slots `ctx` registered.
fn assemble(ctx: &mut Ctx, body: &mut Vec<u8>) {
    let Some(mut slots) = ctx.later.take() else {
        return;
    };
    // Kept on the context with no entries, so a slot started below still
    // inherits the token through `Ctx::child`.
    let entries: Vec<Slot> = slots.entries.drain(..).collect();
    ctx.later = Some(slots);

    let mut running = Vec::new();
    let mut doubled = false;
    for slot in entries {
        match occurrences(body, &slot.marker) {
            1 => running.push(slot),
            0 => {
                let record = crate::floor::note(
                    Level::Warn,
                    "a `Core\\Html::later` placeholder was never written to the page, so its \
                     closure did not run",
                );
                crate::floor::report(ctx, &record);
                release(slot.closure);
            }
            _ => {
                doubled = true;
                release(slot.closure);
            }
        }
    }
    if doubled {
        for slot in running {
            release(slot.closure);
        }
        ctx.set_pending_as(
            ThrownClass::Logic,
            "a `Core\\Html::later` placeholder was written twice; write each one once",
        );
        return;
    }
    if running.is_empty() {
        return;
    }

    let jobs: Vec<Job> = running
        .iter()
        .map(|slot| {
            let closure = slot.closure;
            let deadline = slot.deadline;
            let error = slot.error.clone();
            Box::new(move |child: &mut Ctx| fill(child, closure, deadline, &error)) as Job
        })
        .collect();
    let outcome = crate::host::with_current(|host| host.run_group(ctx, jobs, Bounds::default()));
    let filled = match outcome {
        Some(Outcome::Completed(answers)) => Some(answers),
        Some(Outcome::Fatal(message)) => {
            ctx.set_pending_fatal(message);
            None
        }
        Some(Outcome::Threw(thrown)) => {
            ctx.raise(thrown);
            None
        }
        Some(Outcome::TimedOut | Outcome::Cancelled) => None,
        // An embedder that installed no host: the slots run one after another
        // on this stack, and only their deadlines are lost.
        None => {
            let outer = std::mem::replace(&mut ctx.in_later, true);
            let answers = running
                .iter()
                .map(|slot| render(ctx, slot.closure))
                .collect();
            ctx.in_later = outer;
            Some(answers)
        }
    };
    if let Some(answers) = filled {
        for (slot, answer) in running.iter().zip(answers) {
            let bytes = answer.as_str_bytes().unwrap_or_default().to_vec();
            release(answer);
            if let Some(at) = find(body, &slot.marker) {
                body.splice(at..at + slot.marker.len(), bytes);
            }
        }
    }
    for slot in running {
        release(slot.closure);
    }
}

/// One slot, as a group of one under its own deadline, answering the bytes
/// that replace its placeholder.
fn fill(child: &mut Ctx, closure: Value, deadline: Option<Duration>, error: &[u8]) -> Value {
    // The head is the request's and the main script has finished it, so the
    // slot and every task it starts may not change it.
    child.in_later = true;
    let bounds = Bounds {
        limit: None,
        deadline,
    };
    let outcome = crate::host::with_current(|host| {
        let job: Job = Box::new(move |inner: &mut Ctx| render(inner, closure));
        host.run_group(child, vec![job], bounds)
    });
    match outcome {
        Some(Outcome::Completed(mut answers)) => answers.pop().unwrap_or_default(),
        Some(Outcome::Threw(thrown)) => {
            let mut record = crate::floor::uncaught(&thrown);
            record
                .envelope
                .fields
                .push(("origin".to_owned(), crate::floor::text(ORIGIN)));
            crate::floor::report(child, &record);
            Value::str(NvsStr::new(error))
        }
        Some(Outcome::TimedOut) => {
            let mut record = crate::floor::note(Level::Error, "a `later` slot passed its deadline");
            record
                .envelope
                .fields
                .push(("origin".to_owned(), crate::floor::text(ORIGIN)));
            crate::floor::report(child, &record);
            Value::str(NvsStr::new(error))
        }
        // A limit of the request tree: said on this context, so the outer
        // group stops every sibling and the request fails.
        Some(Outcome::Fatal(message)) => {
            child.set_pending_fatal(message);
            Value::null()
        }
        Some(Outcome::Cancelled) => Value::null(),
        None => render(child, closure),
    }
}

/// Calls the slot's closure and answers its output: what it echoed, then the
/// `Markup` it returned, with its own nested slots already filled.
fn render(ctx: &mut Ctx, closure: Value) -> Value {
    let answer = match crate::call_closure(ctx, closure, &[]) {
        Ok(answer) => answer,
        Err(crate::Fault::Pending(status)) => {
            if status == crate::FATAL {
                ctx.mark_pending_fatal();
            }
            return Value::null();
        }
        Err(crate::Fault::Thrown(class, message)) => {
            ctx.set_pending_as(class, message);
            return Value::null();
        }
        Err(other) => {
            ctx.set_pending(format!("a `later` slot failed: {other:?}"));
            return Value::null();
        }
    };
    let mut bytes = ctx.take_buffered_output().unwrap_or_default();
    if let Some(ptr) = answer.obj_ptr()
        && crate::helpers::is_carrier_value(answer)
    {
        #[expect(
            unsafe_code,
            reason = "the answer is a live carrier this frame owns, so its one \
                      slot is readable, and the read borrows it"
        )]
        // SAFETY: as above.
        let text = unsafe { crate::object::nvs_object_field_get(ptr, crate::CARRIER_TEXT_SLOT) };
        bytes.extend_from_slice(text.as_str_bytes().unwrap_or_default());
    }
    release(answer);
    assemble(ctx, &mut bytes);
    Value::str(NvsStr::new(&bytes))
}

fn occurrences(haystack: &[u8], needle: &[u8]) -> usize {
    haystack
        .windows(needle.len())
        .filter(|window| *window == needle)
        .count()
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn release(value: Value) {
    #[expect(
        unsafe_code,
        reason = "every caller holds exactly the reference it is giving up"
    )]
    // SAFETY: as above.
    unsafe {
        value.release();
    }
}

/// The `origin` field a slot's failure carries in its log record.
const ORIGIN: &str = "a later slot";
