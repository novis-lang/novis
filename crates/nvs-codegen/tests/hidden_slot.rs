//! A revocable object's hidden slot holds a pointer to its memberships, so no
//! value from outside may reach it, and no membership may outlive the
//! collection it names.
//!
//! - `Core\Serialize` writes no node for the slot and reads none: a payload
//!   that names the slot is refused, as any payload of another layout is
//!   (`rule:classes/serialize-is-a-closed-format`).
//! - A decoded revocable object starts in no collection, and joins and leaves
//!   one as a new object does.
//! - A collection write the memory ceiling refused links nothing, so a revoke
//!   after the collection is freed touches no freed memory
//!   (`rule:classes/revocable-objects`).
// covers: lang:classes/revocable-objects

mod common;

use common::*;

/// The class every case here serializes.
const POST: &str = r#"<?nvs
class Post implements Revocable {
    public function constructor(public string $title) {}
}
"#;

/// A payload of one `Post`, written byte by byte: the marker, the version, an
/// object node, its class name, the property names it records, and one node
/// per name. `hidden` puts the hidden slot's name first, with an `int` node
/// as its value, as a payload from an earlier build or a forged one would.
fn post_payload(hidden: bool) -> Vec<u8> {
    fn blob(out: &mut Vec<u8>, bytes: &[u8]) {
        out.push(u8::try_from(bytes.len()).expect("a test name is short"));
        out.extend_from_slice(bytes);
    }
    let mut out = b"NVS\x1b".to_vec();
    out.push(1); // the format version
    out.push(9); // an object node
    blob(&mut out, b"Post");
    if hidden {
        out.push(2);
        blob(&mut out, nvs_runtime::revoke::HIDDEN_SLOT_NAME.as_bytes());
    } else {
        out.push(1);
    }
    blob(&mut out, b"title");
    if hidden {
        // An `int` node whose eight bytes would be read as a pointer.
        out.push(2);
        out.extend_from_slice(&0x4141_4141_4141_4141_u64.to_le_bytes());
    }
    out.push(6); // a string node
    blob(&mut out, b"first");
    out
}

#[test]
fn a_payload_with_a_node_for_the_hidden_slot_is_refused() {
    let unit = compile(POST).expect("the fixture compiles");
    let mut ctx = Ctx::buffered();
    unit.install_in(&mut ctx);
    let entry = unit.script().expect("the script frame was compiled");
    entry.call(&mut ctx).expect("the script ran to completion");

    let refused = nvs_runtime::decode(&post_payload(true), &ctx.receiving())
        .expect_err("a payload with a value for the hidden slot decoded");
    assert!(
        refused
            .message()
            .contains("declares 1 properties and the payload records 2"),
        "the refusal is not the layout's: {}",
        refused.message()
    );

    let built = nvs_runtime::decode(&post_payload(false), &ctx.receiving())
        .expect("a payload of the class's one property decodes");
    #[expect(unsafe_code, reason = "the decode returned one reference, given up here")]
    unsafe {
        built.release();
    }
}

#[test]
fn a_revocable_object_in_a_map_survives_a_round_trip() {
    let source = format!(
        r#"{POST}
class Shelf {{
    public array<Post> $posts = [];
}}
var $first = new Core\ObjectSet<Post>();
Post $post = new Post("first");
$first->add($post);
Shelf $shelf = new Shelf();
$shelf->posts[] = $post;
Shelf $back = Core\Serialize::decode(Core\Serialize::encode($shelf)) as Shelf;
var $second = new Core\ObjectSet<Post>();
foreach ($back->posts as Post $copy) {{
    $second->add($copy);
    echo $copy->title, " ", $second->count();
    $copy->revoke();
    echo " ", $second->count(), " ", $first->count();
}}
"#,
        POST = POST.trim_start_matches("<?nvs\n")
    );
    let source = format!("<?nvs\n{source}");
    // The copy joins a set of its own, and its revoke empties that set and
    // leaves the original's set as it was.
    assert_eq!(output_of(&source), "first 1 0 1");
}

#[test]
#[cfg(debug_assertions)]
fn a_write_the_ceiling_refused_links_nothing() {
    let source = format!(
        r#"{POST}
class K {{
    public static function put(Post $post): void {{
        var $map = new Core\ObjectMap<string, Post>();
        $map->set("k", $post);
    }}
}}
Post $post = new Post("first");
K::put($post);
$post->revoke();
echo "revoked";
"#,
        POST = POST.trim_start_matches("<?nvs\n")
    );
    let source = format!("<?nvs\n{source}");
    let unit = compile(&source).expect("the fixture compiles");
    // The bytes a run holds when the map writes differ by platform, so every
    // ceiling up to well past them is tried. Below them the run stops before
    // the write. Between them and the write's growth the write is refused,
    // `put` frees the map, and the revoke runs before any poll stops the run.
    let mut refused = 0;
    for ceiling in (8..4096).step_by(8) {
        let mut ctx = Ctx::buffered();
        unit.install_in(&mut ctx);
        let entry = unit.script().expect("the script frame was compiled");
        ctx.rebase_memory();
        ctx.set_memory_limit(ceiling);
        let walks = nvs_runtime::budget::membership_walks();
        let revokes = nvs_runtime::revoke::revokes();
        let _ = entry.call(&mut ctx);
        let stopped = ctx
            .take_pending()
            .is_some_and(|message| message.contains("memory"));
        drop(ctx.take_buffered_output());
        if !stopped || nvs_runtime::revoke::revokes() == revokes {
            continue;
        }
        refused += 1;
        // The refused write linked nothing, so the revoke walked no membership.
        assert_eq!(
            nvs_runtime::budget::membership_walks(),
            walks,
            "under a {ceiling}-byte ceiling, a write the ceiling refused left a membership behind"
        );
    }
    assert!(
        refused > 0,
        "no ceiling refused the write and then reached the revoke, so the case proves nothing"
    );
}
