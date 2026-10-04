//! `rule:core-classes/html-later`'s slots: what `Core\Html::later` registered,
//! the placeholder each one wrote, and the two deliveries — one pass that
//! replaces every placeholder with its output, and on a slotted response the
//! shell sent first and each output after it as a fill.
//!
//! # A slot is a registration, and the frame ending is what runs it
//!
//! `Core\Html::later` records its callable here and returns the placeholder. The
//! callables run when the request's own frame has returned, all of them at once
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
//! # A slotted response
//!
//! Where [`Ctx::is_slotted`] holds and the connection offered a
//! [`crate::stream::BodySlot`] carrying its [`Scripts`], the body goes out over
//! that cell as `Core\Response::stream`'s does, with the head the main script
//! declared. The shell is the body up to its last `</body`, placeholders in
//! place. Each slot's task writes its own fill the moment it finishes —
//! `<template for="<name>">…</template>` and the trigger script, the polyfill
//! script ahead of the first one only — so fills arrive in finishing order.
//! One task writes at a time, because a write parks while the peer reads and
//! the cell wakes one writer; a slot that finishes meanwhile queues its fill
//! for the writing task to send. The held-back end goes last, then the body
//! ends, and only then does after-response work run.
//!
//! A limit breach, a throw out of the outer group, or a slot stopped with it
//! leaves a slot without a fill: each one gets its `error` fragment instead,
//! the end is sent, and the request fails as the same page would unslotted.
//! The head is out by then, so the status stays what the main script set.
//! A slot whose task was cancelled while it wrote ends the stream, and the
//! fills after it are not sent.
//!
//! A slotted response with no placeholder to fill is sent whole, and so is
//! one no connection offered a cell to: `nvs run`, a `#[Test]`, an embedder.
//!
//! # What it spends
//!
//! Nothing for a request that never calls `later`: one `Option<Box<_>>` word.
//! A request that does holds one entry per slot — the callable reference, its
//! two fragments and the placeholder bytes — until the pass, and each slot's
//! output until it is spliced. A slotted response holds no whole page: the
//! shell is written once and freed, each fill is held from its slot's end
//! until it is written, and the held-back end until the last fill. All of it
//! is the request's, under its memory cap, and freed with it.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
use std::time::Duration;

use nvs_render::Level;

use crate::ctx::Ctx;
use crate::host::{Bounds, Job, Outcome};
use crate::stream::{BodySlot, Emit, Scripts};
use crate::string::NvsStr;
use crate::throwable::ThrownClass;
use crate::value::Value;

/// What every placeholder begins with, up to its name.
const START: &[u8] = b"<?start name=\"";

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
    callable: Value,
    /// The whole placeholder, as it was handed to the page.
    marker: Vec<u8>,
    /// What the slot shows when its callable throws or passes its deadline.
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
                          callable, and a slot dropped unrun is where it is given back"
            )]
            // SAFETY: the reference was retained for this entry and nothing
            // else points at it.
            unsafe {
                slot.callable.release();
            }
        }
    }
}

impl Ctx {
    /// Registers `callable` as a slot and answers the placeholder bytes the page
    /// writes where its output belongs.
    ///
    /// The caller passes an **owned** reference, which the slot keeps until it
    /// runs or the request ends.
    pub fn register_later(
        &mut self,
        callable: Value,
        placeholder: &[u8],
        error: &[u8],
        deadline: Option<Duration>,
    ) -> Vec<u8> {
        let slots = self.later.get_or_insert_with(|| Box::new(Slots::drawn()));
        let number = slots.next;
        slots.next += 1;
        let mut marker = Vec::with_capacity(placeholder.len() + 48);
        marker.extend_from_slice(START);
        marker.extend_from_slice(b"nvs-");
        marker.extend_from_slice(&slots.token);
        marker.extend_from_slice(format!("-{number}\">").as_bytes());
        marker.extend_from_slice(placeholder);
        marker.extend_from_slice(b"<?end>");
        slots.entries.push(Slot {
            callable,
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

    /// Whether this context runs a slot's callable, or a task inside one. Every
    /// `Core` member that changes the response head reads it and throws
    /// `LogicError` when it is set.
    #[must_use]
    pub fn in_later_slot(&self) -> bool {
        self.in_later
    }

    /// `Core\Response::slotted()`: makes this request's response slotted, or
    /// throws `LogicError` once the main script is over. By then the slots
    /// run, and a normal response is already being assembled.
    ///
    /// # Errors
    ///
    /// A `LogicError` [`crate::Fault`] in a `later` slot, in after-response
    /// work, and in any task either of them starts.
    pub fn make_slotted(&mut self) -> Result<(), crate::Fault> {
        if self.main_ended {
            return Err(crate::Fault::thrown_as(
                ThrownClass::Logic,
                "`Core\\Response::slotted()` was called after the main script ended. Call it \
                 before the main script returns.",
            ));
        }
        self.slotted = true;
        Ok(())
    }

    /// Whether this request's response is slotted: the route it matched was
    /// declared `slotted: true`, or the program called `Core\Response::slotted()`.
    #[must_use]
    pub fn is_slotted(&self) -> bool {
        self.slotted
            || self
                .inbound()
                .and_then(crate::ctx::Inbound::route)
                .is_some_and(|matched| matched.route().slotted())
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
    // Before the early return: a request with no slot still has a main script
    // that is over, and its after-response work reads this.
    ctx.main_ended = true;
    if !ctx.has_later() || ctx.pending().is_some() || ctx.ending().is_err() {
        return;
    }
    let Some(mut body) = ctx.take_buffered_output() else {
        return;
    };
    if let Some((cell, scripts)) = slotted_cell(ctx) {
        deliver(ctx, body, &cell, scripts);
        return;
    }
    assemble(ctx, &mut body);
    ctx.restore_body(body);
}

/// The pass itself, over `body`, for the slots `ctx` registered.
fn assemble(ctx: &mut Ctx, body: &mut Vec<u8>) {
    let Some(running) = claim(ctx, body) else {
        return;
    };
    let jobs: Vec<Job> = running
        .iter()
        .map(|slot| {
            let callable = slot.callable;
            let deadline = slot.deadline;
            let error = slot.error.clone();
            Box::new(move |child: &mut Ctx| fill(child, callable, deadline, &error)) as Job
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
                .map(|slot| render(ctx, slot.callable))
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
        release(slot.callable);
    }
}

/// The slots `ctx` registered whose placeholder `body` carries once, taken off
/// the context, and `None` where there is nothing to run.
///
/// A placeholder written nowhere cancels its slot with a `Warn`. One written
/// twice cancels every slot and leaves `LogicError` pending.
fn claim(ctx: &mut Ctx, body: &[u8]) -> Option<Vec<Slot>> {
    let mut slots = ctx.later.take()?;
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
                     callable did not run",
                );
                crate::floor::report(ctx, &record);
                release(slot.callable);
            }
            _ => {
                doubled = true;
                release(slot.callable);
            }
        }
    }
    if doubled {
        for slot in running {
            release(slot.callable);
        }
        ctx.set_pending_as(
            ThrownClass::Logic,
            "a `Core\\Html::later` placeholder was written twice; write each one once",
        );
        return None;
    }
    (!running.is_empty()).then_some(running)
}

/// The cell a slotted page is sent through, and what it sends beside its
/// fills: `None` where the response is not slotted, or where it cannot be sent
/// over time — no host to park on, no connection, or a body already opened.
fn slotted_cell(ctx: &Ctx) -> Option<(BodySlot, Scripts)> {
    if !ctx.is_slotted() || crate::host::with_current(|_| ()).is_none() {
        return None;
    }
    let cell = ctx
        .inbound()
        .and_then(crate::ctx::Inbound::response_stream_slot)?;
    let scripts = cell.scripts()?;
    (!cell.is_open()).then(|| (cell.clone(), scripts))
}

/// The slotted delivery: `body` goes out as the shell, without its
/// `</body>` and what follows, then each slot's output as a `<template for>`
/// fill in the order the slots finish, then the held-back end.
fn deliver(ctx: &mut Ctx, mut body: Vec<u8>, cell: &BodySlot, scripts: Scripts) {
    let Some(running) = claim(ctx, &body) else {
        ctx.restore_body(body);
        return;
    };
    let content_type = ctx.take_content_type();
    let media_type = content_type
        .as_deref()
        .unwrap_or(crate::host::ECHOED_MEDIA_TYPE);
    let status = ctx.take_status();
    let headers = ctx.take_headers();
    // `slotted_cell` checked the cell is unopened, so this always opens it.
    let Some(mut emit) = cell.open_slotted(media_type, status, headers) else {
        ctx.restore_body(body);
        return;
    };
    let end = body.split_off(held_back(&body));
    // A client that has gone is not the program's error, and the slots run on
    // all the same (`rule:http-server/a-request-outlives-a-client-that-goes-away`).
    let _ = emit.send_all(body);

    let outbox = Rc::new(RefCell::new(Outbox {
        emit: Some(emit),
        queue: VecDeque::new(),
        filled: vec![false; running.len()],
        polyfill: Some(scripts.polyfill),
        trigger: scripts.trigger,
    }));
    let jobs: Vec<Job> = running
        .iter()
        .enumerate()
        .map(|(index, slot)| {
            let callable = slot.callable;
            let deadline = slot.deadline;
            let error = slot.error.clone();
            let name = name_of(&slot.marker).to_vec();
            let outbox = Rc::clone(&outbox);
            Box::new(move |child: &mut Ctx| {
                let answer = fill(child, callable, deadline, &error);
                // A string is a finished slot. `null` is a slot stopped by a
                // limit or cancelled, which the pass below fills instead.
                if let Some(bytes) = answer.as_str_bytes() {
                    post(&outbox, index, &name, bytes);
                    pump(&outbox);
                }
                release(answer);
                Value::null()
            }) as Job
        })
        .collect();
    let outcome = crate::host::with_current(|host| host.run_group(ctx, jobs, Bounds::default()));
    let failed = match outcome {
        Some(Outcome::Fatal(message)) => {
            ctx.set_pending_fatal(message);
            true
        }
        Some(Outcome::Threw(thrown)) => {
            ctx.raise(thrown);
            true
        }
        _ => false,
    };
    // The head is already out, so a limit breach cannot change the status.
    // Every slot still open shows its `error` fragment, and the page ends.
    if failed {
        for (index, slot) in running.iter().enumerate() {
            post(&outbox, index, name_of(&slot.marker), &slot.error);
        }
    }
    pump(&outbox);
    if let Some(mut emit) = outbox.borrow_mut().emit.take() {
        let _ = emit.send_all(end);
        emit.finish();
    }
    for slot in running {
        release(slot.callable);
    }
}

/// What the slots of one slotted page share while they run: the writing half,
/// the fills waiting for it, and which slots have one.
struct Outbox {
    /// `None` while a slot's task is writing. Only one task writes at a time,
    /// because a write can park and the cell wakes one writer.
    emit: Option<Emit>,
    /// Fills in the order their slots finished, not yet written.
    queue: VecDeque<Vec<u8>>,
    filled: Vec<bool>,
    /// Taken by the first fill, which is the only one that carries it.
    polyfill: Option<&'static str>,
    trigger: &'static str,
}

/// Queues the fill for slot `index`, unless it already has one.
fn post(outbox: &RefCell<Outbox>, index: usize, name: &[u8], content: &[u8]) {
    let mut outbox = outbox.borrow_mut();
    if std::mem::replace(&mut outbox.filled[index], true) {
        return;
    }
    let mut chunk = Vec::with_capacity(content.len() + 96);
    if let Some(polyfill) = outbox.polyfill.take() {
        chunk.extend_from_slice(b"<script>");
        chunk.extend_from_slice(polyfill.as_bytes());
        chunk.extend_from_slice(b"</script>");
    }
    chunk.extend_from_slice(b"<template for=\"");
    chunk.extend_from_slice(name);
    chunk.extend_from_slice(b"\">");
    chunk.extend_from_slice(content);
    chunk.extend_from_slice(b"</template><script>");
    chunk.extend_from_slice(outbox.trigger.as_bytes());
    chunk.extend_from_slice(b"</script>");
    outbox.queue.push_back(chunk);
}

/// Writes every queued fill, unless another task is already writing: that
/// task writes this one too before it gives the writing half back.
fn pump(outbox: &RefCell<Outbox>) {
    loop {
        let (mut emit, chunk) = {
            let mut outbox = outbox.borrow_mut();
            let Some(emit) = outbox.emit.take() else {
                return;
            };
            let Some(chunk) = outbox.queue.pop_front() else {
                outbox.emit = Some(emit);
                return;
            };
            (emit, chunk)
        };
        // A client that has gone: the fill is dropped and the slots run on.
        let _ = emit.send_all(chunk);
        outbox.borrow_mut().emit = Some(emit);
    }
}

/// The slot's name, `nvs-<token>-<n>`, out of its whole placeholder.
fn name_of(marker: &[u8]) -> &[u8] {
    let name = &marker[START.len()..];
    let end = name.iter().position(|&byte| byte == b'"').unwrap_or(0);
    &name[..end]
}

/// Where the part of the page sent after the last fill begins: the last
/// `</body`, in any case, or the end of the page where it has none.
fn held_back(body: &[u8]) -> usize {
    const CLOSE: &[u8] = b"</body";
    body.windows(CLOSE.len())
        .rposition(|window| window.eq_ignore_ascii_case(CLOSE))
        .unwrap_or(body.len())
}

/// One slot, as a group of one under its own deadline, answering the bytes
/// that replace its placeholder.
fn fill(child: &mut Ctx, callable: Value, deadline: Option<Duration>, error: &[u8]) -> Value {
    // The head is the request's and the main script has finished it, so the
    // slot and every task it starts may not change it.
    child.in_later = true;
    let bounds = Bounds {
        limit: None,
        deadline,
    };
    let outcome = crate::host::with_current(|host| {
        let job: Job = Box::new(move |inner: &mut Ctx| render(inner, callable));
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
        None => render(child, callable),
    }
}

/// Calls the slot's callable and answers its output: what it echoed, then the
/// `Markup` it returned, with its own nested slots already filled.
fn render(ctx: &mut Ctx, callable: Value) -> Value {
    let answer = match crate::call_callable(ctx, callable, &[]) {
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
