//! A request tree's budget, read from a core the tree's root does not run on —
//! `nvs_runtime::budget::OffCore` and the two `Ctx` seams that reach it.
//!
//! `rule:security/isolate-budget-is-the-trees` gives a request and everything it
//! spawns one budget accounted at the tree's root, and the counters that answer
//! it are thread-local: a request is charged the difference between its thread's
//! balance now and the balance when its `Ctx` was made. That reads as the whole
//! tree's only while the whole tree is one core's. A child placed on another
//! core (`rule:concurrency/on-worker-runs-the-child-on-another-core`) takes its
//! zero point *there*, so without the pair these cases ask about, a tree would
//! hold one ceiling per core it reached.
//!
//! The cases ask the crossing from both ends — that a member off the root's core
//! lands on the root's reading while it runs, that it gives its memory back and
//! keeps its writing when it ends, that it does not count itself twice, and that
//! the pair belongs to one tree rather than to the process. They ask the
//! mechanism with an ordinary `std::thread`, because that is the boundary: what
//! `nvs-host` adds on top is a scheduler on the far side, not a different
//! question about the counters.

use std::sync::mpsc;

use nvs_runtime::Ctx;

/// Big enough that no allocation the test harness happens to make on either
/// thread can be mistaken for it, and small enough to hold twice over.
const HELD: usize = 8 << 20;

/// What a member off the root's core writes — a number, not a shape: the case
/// asserts the count and never the bytes.
const WRITTEN: &[u8] = b"answered from another core";

/// A ceiling wide enough that [`HELD`] is a visible part of it and not all of
/// it, so a sub-cap taken against it is still a number a child could run under.
const ROOT_LIMIT: usize = 4 * HELD;

/// The shape every case here takes: a member of `root`'s tree, started on
/// another thread, that holds `HELD` bytes and writes `WRITTEN` until it is told
/// to end.
///
/// It hands back the channel that releases it and the join handle, so a case can
/// read the root's counters while the member is still alive — which is the
/// interval the whole arrangement exists for.
fn member_on_another_thread(root: &Ctx) -> (mpsc::Sender<()>, std::thread::JoinHandle<usize>) {
    let handle = root.tree_handle();
    let (published, ready) = mpsc::channel();
    let (release, released) = mpsc::channel();
    let thread = std::thread::spawn(move || {
        let mut member = Ctx::buffered();
        member.join_tree(handle);
        let held = vec![7_u8; HELD];
        member
            .write_output(WRITTEN)
            .expect("a buffer sink takes it");
        // The poll `crate::run_helper` makes ahead of every `Core` member, which
        // is where a member off the root's core publishes its share.
        let _ = member.memory_breach();
        let own = member.memory_used();
        published.send(own).expect("the case is still waiting");
        released.recv().expect("the case releases its member");
        drop(held);
        drop(member);
        own
    });
    let own = ready.recv().expect("the member publishes before it parks");
    assert!(
        own >= HELD,
        "a member reads its own thread's balance: {own} bytes against {HELD} held"
    );
    (release, thread)
}

/// The reading the tree's ceiling is enforced against holds a member running on
/// another core, while that member is still running.
///
/// This is the property `rule:security/isolate-budget-is-the-trees` cannot do
/// without: a root whose reading stopped at its own thread would hand a request
/// one whole ceiling per core it placed a child on, which is a fork bomb bounded
/// by nothing.
#[test]
fn a_root_reads_the_memory_and_the_output_of_a_member_on_another_thread() {
    let root = Ctx::buffered();
    let memory_before = root.memory_used();
    let output_before = root.output_used();

    let (release, thread) = member_on_another_thread(&root);

    let memory = root.memory_used();
    assert!(
        memory >= memory_before + HELD,
        "the root reads {memory} bytes, which does not hold the member's {HELD}"
    );
    let output = root.output_used();
    assert_eq!(
        output,
        output_before + WRITTEN.len(),
        "the root's writing does not hold the member's"
    );

    release.send(()).expect("the member is waiting");
    thread.join().expect("the member ends cleanly");
}

/// The two counters part when the member ends: its memory comes off the tree's
/// reading and its writing stays on it.
///
/// That is not a detail of this implementation but the difference between the
/// two directives. `[limits] memory` bounds what a tree *holds*, and a member
/// that has ended holds nothing; `[limits] max_output` bounds what it has
/// *written*, and a response cannot be unwritten.
#[test]
fn a_member_that_ends_gives_its_memory_back_and_keeps_its_writing() {
    let root = Ctx::buffered();
    let memory_before = root.memory_used();
    let output_before = root.output_used();

    let (release, thread) = member_on_another_thread(&root);
    release.send(()).expect("the member is waiting");
    thread.join().expect("the member ends cleanly");

    let memory = root.memory_used();
    assert!(
        memory < memory_before + HELD,
        "the root still reads {memory} bytes for a member that has ended"
    );
    assert_eq!(
        root.output_used(),
        output_before + WRITTEN.len(),
        "the member's writing left the tree's count when the member did"
    );
}

/// The pair is the *tree's*, so one tree's member is invisible to another tree
/// running on the very thread that reads it.
///
/// A process-wide counter would answer this case wrongly in the direction that
/// matters: every request on the machine would be charged for every other
/// request's off-core children, and the first one to poll would be stopped for
/// allocation it never made.
#[test]
fn one_trees_member_is_not_on_another_trees_reading() {
    let root = Ctx::buffered();
    let stranger = Ctx::buffered();
    let before = stranger.memory_used();

    let (release, thread) = member_on_another_thread(&root);

    assert!(
        stranger.memory_used() < before + HELD,
        "a second tree on this thread was charged for the first tree's member"
    );

    release.send(()).expect("the member is waiting");
    thread.join().expect("the member ends cleanly");
}

/// A member off the root's core reads its own thread and adds the pair it is
/// publishing into to nothing.
///
/// Double counting here would be the ordinary case rather than an edge: the
/// member publishes its whole share every poll, so a reading that added the pair
/// back would report twice what the member holds and refuse it at half its
/// sub-cap.
#[test]
fn a_member_does_not_read_the_share_it_published() {
    let root = Ctx::buffered();
    let (release, thread) = member_on_another_thread(&root);
    let own = {
        release.send(()).expect("the member is waiting");
        thread.join().expect("the member ends cleanly")
    };
    assert!(
        own < 2 * HELD,
        "a member read {own} bytes for the {HELD} it holds — its own published share counted twice"
    );
}

/// `Ctx::placed_isolate` crosses a thread, and the context it builds there is a
/// member of the tree it came from, under a ceiling of what remained of that
/// tree's budget at the spawn.
///
/// The two halves are one case because neither is worth anything alone. A seed
/// that crossed without joining would give the child a budget of its own, which
/// is the fork bomb the rule above exists to bound; a child that joined under the
/// *root's* whole ceiling would be handed the budget its parent had already
/// spent.
#[test]
fn a_placed_seed_builds_a_member_of_its_tree_under_what_remained_of_its_budget() {
    let mut root = Ctx::buffered();
    root.set_memory_limit(ROOT_LIMIT);
    let spent = vec![3_u8; HELD];
    let used = root.memory_used();

    let seed = root.placed_isolate();
    let (published, ready) = mpsc::channel();
    let (release, released) = mpsc::channel();
    let thread = std::thread::spawn(move || {
        let child = seed.build(nvs_runtime::OutputSink::Buffer(Vec::new()));
        let held = vec![5_u8; HELD];
        let _ = child.memory_breach();
        published
            .send(child.memory_limit())
            .expect("the case is still waiting");
        released.recv().expect("the case releases its child");
        drop(held);
    });

    let cap = ready.recv().expect("the child publishes before it parks");
    assert!(
        cap <= ROOT_LIMIT - used && cap > 0,
        "a child placed with {used} of {ROOT_LIMIT} bytes spent was capped at {cap}"
    );
    let reading = root.memory_used();
    assert!(
        reading >= used + HELD,
        "the root reads {reading} bytes, which does not hold the placed child's {HELD}"
    );

    release.send(()).expect("the child is waiting");
    thread.join().expect("the child ends cleanly");
    drop(spent);
}

/// A tree with nothing left of its budget places a child under a ceiling it
/// cannot allocate under, and never under none at all.
///
/// Zero is the sentinel for *no ceiling*, so the subtraction that computes a
/// sub-cap has one answer it may not give: a tree that has spent everything
/// handing its child an unbounded budget is the one arithmetic mistake that fails
/// open rather than shut.
#[test]
fn a_tree_with_nothing_left_places_a_child_under_a_ceiling_rather_than_under_none() {
    let mut root = Ctx::buffered();
    root.set_memory_limit(1);
    let _spent = vec![9_u8; HELD];

    let cap = root.placed_isolate().build(nvs_runtime::OutputSink::Sink);

    assert_eq!(
        cap.memory_limit(),
        1,
        "a spent tree placed a child under a ceiling of {} bytes",
        cap.memory_limit()
    );
}
