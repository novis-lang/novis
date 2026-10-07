//! `nvs meta --json`, driven as a consumer drives it —
//! `rule:core-api/reference-card`'s
//! *Verification* section: a golden of § 2's shape over the first documented
//! member, through the built binary for the same reason `openapi.rs` goes
//! through it — the contract is what the *command* prints.

use std::process::Command;

/// `nvs meta <args...>`, as `(stdout, success)`.
fn meta(args: &[&str]) -> (String, bool) {
    let out = Command::new(env!("CARGO_BIN_EXE_nvs"))
        .arg("meta")
        .args(args)
        .output()
        .expect("the `nvs` binary this test was built beside runs");
    (
        String::from_utf8(out.stdout).expect("the document is UTF-8"),
        out.status.success(),
    )
}

/// The parsed document, and the one member of it `class::member` names.
fn member(document: &serde_json::Value, class: &str, member: &str) -> serde_json::Value {
    document["classes"]
        .as_array()
        .expect("`classes` is an array")
        .iter()
        .find(|c| c["name"] == class)
        .unwrap_or_else(|| panic!("{class} is in the document"))["members"]
        .as_array()
        .expect("`members` is an array")
        .iter()
        .find(|m| m["name"] == member)
        .unwrap_or_else(|| panic!("{class}::{member} is in the document"))
        .clone()
}

/// The parsed document's top-level enum `name` names.
fn core_enum(document: &serde_json::Value, name: &str) -> serde_json::Value {
    document["enums"]
        .as_array()
        .expect("`enums` is an array")
        .iter()
        .find(|e| e["name"] == name)
        .unwrap_or_else(|| panic!("{name} is in the document"))
        .clone()
}

/// The parsed document's constant `class::name` names.
fn constant(document: &serde_json::Value, class: &str, name: &str) -> serde_json::Value {
    document["classes"]
        .as_array()
        .expect("`classes` is an array")
        .iter()
        .find(|c| c["name"] == class)
        .unwrap_or_else(|| panic!("{class} is in the document"))["constants"]
        .as_array()
        .expect("`constants` is an array")
        .iter()
        .find(|c| c["name"] == name)
        .unwrap_or_else(|| panic!("{class}::{name} is in the document"))
        .clone()
}

/// § 2's golden: `Core\Str::length`, the simple case — the `$name` its one
/// parameter is callable by, a `short`, one documented parameter, a `return`,
/// and no `errors` or `shape` key because neither is written.
#[test]
fn the_golden_for_str_length_matches_the_contract() {
    let (doc, ok) = meta(&["--json"]);
    assert!(ok, "`nvs meta --json` succeeds");
    let document: serde_json::Value = serde_json::from_str(&doc).expect("the document is JSON");
    assert_eq!(
        member(&document, r"Core\Str", "length"),
        serde_json::json!({
            "name": "length",
            "kind": "static",
            "names": ["s"],
            "signature": "length(string $s): uint",
            "params": [ { "name": "s", "type": "string", "qualifier": "neutral" } ],
            "returns": "uint",
            "doc": {
                "short": "Counts the graphemes in `$s` — user-perceived characters, the unit \
                          every `Core\\Str` member counts in — so a combining sequence counts \
                          once and this is never a byte count.",
                "params": [ { "name": "s", "desc": "The string to measure." } ],
                "return": "The grapheme count; `0` for the empty string."
            }
        })
    );
}

/// Every member the document lists carries a `doc` with a `short` — the
/// registry's own guard `every_registry_row_carries_a_reference_card` seen
/// from the consumer's side, so a card the emitter dropped on the way out
/// would show here. `names` is beside it on every row, because `rule:core-api/shape-rules` R2's
/// by-name surface is signature and not documentation. The omission rule
/// itself — no key for a field with nothing written — is proven over
/// synthetic cards in `meta.rs`'s own tests, because every shipped row
/// carries a card.
#[test]
fn every_member_carries_a_doc_key() {
    let (doc, _) = meta(&["--json"]);
    let document: serde_json::Value = serde_json::from_str(&doc).expect("the document is JSON");
    let mut undocumented = Vec::new();
    for class in document["classes"]
        .as_array()
        .expect("`classes` is an array")
    {
        for m in class["members"].as_array().expect("`members` is an array") {
            // `names` is absent, not empty, on a member with no parameter —
            // the omission rule again — so it is not a thing to require here.
            let written = m["doc"]["short"].as_str().is_some_and(|s| !s.is_empty());
            if !written {
                undocumented.push(format!("{}::{}", class["name"], m["name"]));
            }
        }
    }
    assert!(
        undocumented.is_empty(),
        "members without a card: {undocumented:?}"
    );
}

/// The other members documented as proof: an options bag is one `params`
/// entry per option under the option's own name, and a thrown error is an
/// `errors` entry naming the class a `catch` writes.
#[test]
fn the_options_and_errors_of_the_proof_members_are_emitted() {
    let (doc, _) = meta(&["--json"]);
    let document: serde_json::Value = serde_json::from_str(&doc).expect("the document is JSON");

    let encode = member(&document, r"Core\Json", "encode");
    let names: Vec<&str> = encode["doc"]["params"]
        .as_array()
        .expect("params")
        .iter()
        .map(|p| p["name"].as_str().expect("a name"))
        .collect();
    assert_eq!(names, ["value", "pretty", "escapeUnicode"]);
    assert_eq!(encode["doc"]["errors"][0]["error"], "LogicError");

    let matched = member(&document, r"Core\Regex", "match");
    let errors = matched["doc"]["errors"].as_array().expect("errors");
    assert_eq!(
        errors.len(),
        1,
        "one entry per class thrown, its conditions in the desc"
    );
    assert_eq!(errors[0]["error"], "RuntimeError");
}

/// § 2's enum golden: `Core\Order`, top-level beside `classes`, a `short` and
/// one `cases` entry per case in declaration order.
#[test]
fn the_golden_for_order_matches_the_contract() {
    let (doc, _) = meta(&["--json"]);
    let document: serde_json::Value = serde_json::from_str(&doc).expect("the document is JSON");
    assert_eq!(
        core_enum(&document, r"Core\Order"),
        serde_json::json!({
            "name": r"Core\Order",
            "doc": {
                "short": "The direction `Core\\Arr::sort` and `sortByKey` put elements in — spec \
                          § 2's `{order: …}` option, which is `Asc` when omitted.",
                "cases": [
                    { "name": "Asc", "desc": "Smallest first — what an omitted `{order: …}` means." },
                    { "name": "Desc", "desc": "Largest first — `rsort`, `arsort` and `krsort` as \
                                               one option rather than three names." }
                ]
            }
        })
    );
}

/// § 2's constant golden: `Core\Math::PI`, under its class's `constants`, its
/// one-sentence card as `doc`.
#[test]
fn the_golden_for_math_pi_matches_the_contract() {
    let (doc, _) = meta(&["--json"]);
    let document: serde_json::Value = serde_json::from_str(&doc).expect("the document is JSON");
    assert_eq!(
        constant(&document, r"Core\Math", "PI"),
        serde_json::json!({
            "name": "PI",
            "type": "float",
            "value": "3.141592653589793",
            "doc": "The ratio of a circle's circumference to its diameter, `3.14159…` as the \
                    nearest `float`."
        })
    );
}

/// Every enum and every constant carries its card too — the same guard, on
/// the rosters a member's card points at. `Core\SetOn` and `Core\Math::TAU` are
/// named outright beside the walk, so one entry of each roster is asserted
/// directly and a roster that emptied would fail here rather than pass over
/// nothing.
#[test]
fn every_enum_and_constant_carries_a_doc_key() {
    let (doc, _) = meta(&["--json"]);
    let document: serde_json::Value = serde_json::from_str(&doc).expect("the document is JSON");
    assert!(core_enum(&document, r"Core\SetOn")["doc"]["short"].is_string());
    assert!(constant(&document, r"Core\Math", "TAU")["doc"].is_string());
    let mut undocumented = Vec::new();
    for declared in document["enums"].as_array().expect("`enums` is an array") {
        if !declared["doc"]["short"].is_string() {
            undocumented.push(declared["name"].to_string());
        }
    }
    for class in document["classes"]
        .as_array()
        .expect("`classes` is an array")
    {
        // A class with no constants has no `constants` key at all.
        let Some(constants) = class["constants"].as_array() else {
            continue;
        };
        for c in constants {
            if !c["doc"].is_string() {
                undocumented.push(format!("{}::{}", class["name"], c["name"]));
            }
        }
    }
    assert!(
        undocumented.is_empty(),
        "enums and constants without a card: {undocumented:?}"
    );
}

/// The built-in image component's classes follow the registry's in `classes`:
/// `Novis\Image\Image` and `Novis\Image\Color` from its Novis source with their
/// public members only, and `Novis\Image\Codec` from its manifest with every
/// export `static`. Its enums follow the registry's in `enums`, under the
/// export class's namespace, with their cases.
// covers: tools:cli/nvs-meta-json
#[test]
fn meta_lists_the_built_in_components_classes_and_enums() {
    let (doc, ok) = meta(&["--json"]);
    assert!(ok, "`nvs meta --json` succeeds");
    let document: serde_json::Value = serde_json::from_str(&doc).expect("the document is JSON");
    let names = |class: &str| -> Vec<String> {
        document["classes"]
            .as_array()
            .expect("`classes` is an array")
            .iter()
            .find(|c| c["name"] == class)
            .unwrap_or_else(|| panic!("{class} is in the document"))["members"]
            .as_array()
            .expect("`members` is an array")
            .iter()
            .map(|m| m["name"].as_str().expect("a member has a name").to_owned())
            .collect()
    };
    let image = names(r"Novis\Image\Image");
    for public in [
        "open", "create", "fromRaw", "resize", "encode", "variants", "raw",
    ] {
        assert!(
            image.contains(&public.to_owned()),
            "Image::{public} is listed"
        );
    }
    for private in ["constructor", "then", "lastFormat", "rgba"] {
        assert!(
            !image.contains(&private.to_owned()),
            "Image::{private} is private and not listed"
        );
    }
    assert_eq!(names(r"Novis\Image\Color"), ["rgba", "hex"]);
    assert_eq!(names(r"Novis\Image\Codec"), ["info", "run", "variants"]);
    assert_eq!(
        member(&document, r"Novis\Image\Codec", "info")["kind"],
        "static"
    );
    let signature = member(&document, r"Novis\Image\Codec", "info")["signature"].clone();
    assert!(
        signature
            .as_str()
            .is_some_and(|s| s.starts_with("info(bytes $data): {format: Format")),
        "{signature}"
    );
    assert_eq!(
        member(&document, r"Novis\Image\Image", "resize")["kind"],
        "instance"
    );
    let fit = core_enum(&document, r"Novis\Image\Fit");
    assert_eq!(fit["cases"][0]["name"], "Cover");
    for name in ["Format", "Gravity", "Filter", "Axis", "Blend"] {
        core_enum(&document, &format!(r"Novis\Image\{name}"));
    }
}

/// A built-in member's card is its source's `///` run or its manifest's
/// `help`, under `doc.short` as a `Core` member's is, and an enum's card is
/// its manifest `help`. `every_member_carries_a_doc_key` above then holds every
/// built-in member to a card too, because it walks the same `classes`.
// covers: tools:cli/nvs-meta-json
#[test]
fn a_built_in_member_carries_its_card() {
    let (doc, _) = meta(&["--json"]);
    let document: serde_json::Value = serde_json::from_str(&doc).expect("the document is JSON");
    let short = |value: serde_json::Value| value["doc"]["short"].as_str().map(str::to_owned);
    let open = short(member(&document, r"Novis\Image\Image", "open")).expect("open has a card");
    assert!(
        open.starts_with("`Image::open` starts from the bytes"),
        "{open}"
    );
    assert!(
        !open.contains("///"),
        "the marker is not part of the card: {open}"
    );
    assert_eq!(
        short(member(&document, r"Novis\Image\Codec", "info")).as_deref(),
        Some(
            "Reads the format, the size and the orientation of an encoded image from its header. No pixel is decoded."
        )
    );
    assert!(short(core_enum(&document, r"Novis\Image\Fit")).is_some());
    let class = document["classes"]
        .as_array()
        .expect("`classes` is an array")
        .iter()
        .find(|c| c["name"] == r"Novis\Image\Image")
        .expect("Image is in the document")
        .clone();
    assert!(short(class).is_some(), "the class carries its own card");
}

/// `--json` is required: a `meta` that prints nothing would be a subcommand
/// that succeeds having done nothing, exactly as `build` without `--openapi`.
// covers: tools:cli/nvs-meta-json
#[test]
fn meta_without_json_is_refused() {
    let (_, ok) = meta(&[]);
    assert!(!ok);
}

/// The fixture program `rule:tooling/meta-json-takes-a-program`'s tests read,
/// as a path this binary can be handed from any working directory.
///
/// Built from `CARGO_MANIFEST_DIR` because the fixture's own `@example` paths
/// resolve against the file that wrote them, so the entry point has to be the
/// committed file rather than a copy somewhere else.
fn fixture() -> String {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/meta/program.nvs")
        .display()
        .to_string()
}

/// `program`, the one key an entry point adds.
fn program(args: &[&str]) -> serde_json::Value {
    let (doc, ok) = meta(args);
    assert!(ok, "`nvs meta {args:?}` succeeds");
    let document: serde_json::Value = serde_json::from_str(&doc).expect("the document is JSON");
    document["program"].clone()
}

/// The declaration `roster::name` names in the program half.
fn declared(program: &serde_json::Value, roster: &str, name: &str) -> serde_json::Value {
    program[roster]
        .as_array()
        .unwrap_or_else(|| panic!("`{roster}` is an array"))
        .iter()
        .find(|d| d["name"] == name)
        .unwrap_or_else(|| panic!("{name} is in `{roster}`"))
        .clone()
}

/// The seam `rule:tooling/meta-json-takes-a-program` states: the argument adds
/// one key and changes nothing else, so the two renderers already on this
/// pipeline — `bun nv reference` and the website's Core reference — read the
/// same bytes they did before it existed.
///
/// Asserted by taking the program half back out of the entry-point document and
/// comparing what is left, byte for byte, against the no-argument one. A
/// document that had been reshaped anywhere — a key renamed, a roster moved
/// under the program, a field emitted only when a program is named — fails here
/// while both halves still look right read on their own.
// covers: tools:cli/nvs-meta-json
#[test]
fn meta_json_with_no_argument_is_byte_identical_to_the_registry_dump() {
    let (registry, ok) = meta(&["--json"]);
    assert!(ok, "`nvs meta --json` succeeds");
    let (with_program, ok) = meta(&["--json", &fixture()]);
    assert!(ok, "`nvs meta --json <entry>` succeeds");

    let mut document: serde_json::Value =
        serde_json::from_str(&with_program).expect("the document is JSON");
    let removed = document
        .as_object_mut()
        .expect("the document is an object")
        .remove("program");
    assert!(removed.is_some(), "an entry point adds a `program` key");
    assert_eq!(format!("{document}\n"), registry);
}

/// An entry point's own declarations reach the document, in every roster the
/// program fills and in the shape the registry half already uses: a class with
/// `members` and `constants`, an enum with `cases`, and a `type` alias with the
/// type it stands for.
// covers: tools:cli/nvs-meta-json
#[test]
fn meta_json_with_an_entry_emits_the_programs_declarations() {
    let program = program(&["--json", &fixture()]);

    let greeter = declared(&program, "classes", "Greeter");
    let members: Vec<(&str, &str)> = greeter["members"]
        .as_array()
        .expect("`members` is an array")
        .iter()
        .map(|m| {
            (
                m["name"].as_str().expect("a name"),
                m["kind"].as_str().expect("a kind"),
            )
        })
        .collect();
    assert_eq!(
        members,
        vec![
            ("$last", "property"),
            ("constructor", "constructor"),
            ("greet", "instance"),
            ("anyone", "static"),
        ]
    );
    assert_eq!(
        declared(&greeter, "constants", "OPENING"),
        serde_json::json!({
            "name": "OPENING",
            "visibility": "public",
            "type": "string",
            "value": "\"Hello\"",
            "doc": { "short": "The word every greeting opens with." },
        })
    );
    assert_eq!(
        greeter["members"][2]["signature"],
        "greet(string $who): string"
    );

    let volume = declared(&program, "enums", "Volume");
    assert_eq!(volume["backing"], "int");
    let cases: Vec<&str> = volume["cases"]
        .as_array()
        .expect("`cases` is an array")
        .iter()
        .map(|c| c["name"].as_str().expect("a name"))
        .collect();
    assert_eq!(cases, vec!["Quiet", "Normal"]);

    assert_eq!(declared(&program, "types", "Greeting")["type"], "string");
}

/// An alias a body owns is emitted inside its owner's entry, under the same
/// `types` key and in the same card shape the file-scope form already has —
/// named `Owner::Name`, which is the spelling a program reaches it by
/// (`rule:types/type-alias`).
///
/// A class and an enum both own one here, because the member is one rule for
/// every body that has a class-shaped name. What the two rosters must not do is
/// merge: the owner's aliases are its own and the file-scope roster holds only
/// what was written at file scope, so a consumer asking "what does this program
/// declare" gets each name exactly once.
#[test]
fn meta_json_nests_a_class_scoped_alias_under_its_owner() {
    let program = program(&["--json", &fixture()]);

    let greeter = declared(&program, "classes", "Greeter");
    assert_eq!(
        declared(&greeter, "types", "Greeter::Card"),
        serde_json::json!({
            "name": "Greeter::Card",
            "type": "{text: string, volume: int}",
            "doc": { "short": "What a rendered greeting is: its text and how loudly to say it." },
        })
    );
    // An alias is not a member: it declares no value, takes no visibility and is
    // gone by runtime, so it never joins the roster a property and a method
    // share.
    let names: Vec<&str> = greeter["members"]
        .as_array()
        .expect("`members` is an array")
        .iter()
        .map(|m| m["name"].as_str().expect("a name"))
        .collect();
    assert!(!names.contains(&"Card"), "an alias is not in `members`");

    // The enum's is undocumented, so it carries no `doc` key — the registry's
    // omission rule, which the second declaration site inherits whole.
    assert_eq!(
        declared(
            &declared(&program, "enums", "Volume"),
            "types",
            "Volume::Pair"
        ),
        serde_json::json!({ "name": "Volume::Pair", "type": "array<Volume>" })
    );

    // And the file-scope roster is still only what file scope declared.
    let file_scope: Vec<&str> = program["types"]
        .as_array()
        .expect("`types` is an array")
        .iter()
        .map(|t| t["name"].as_str().expect("a name"))
        .collect();
    assert_eq!(file_scope, vec!["Greeting"]);
}

/// A user declaration's card is the `///` run above it: its prose under
/// `short`, its `@see` targets and its `@example` paths as their own lists, and
/// the registry's omission rule over all three — an undocumented declaration
/// has no `doc` key, and neither tag list is ever emitted empty.
#[test]
fn a_user_declarations_shape_carries_prose_see_and_example() {
    let program = program(&["--json", &fixture()]);
    let greeter = declared(&program, "classes", "Greeter");

    assert_eq!(
        greeter["doc"],
        serde_json::json!({
            "short": "Greets somebody by name.",
            "see": ["Greeter::greet"],
            "example": ["examples/greeting.nvs"],
        })
    );
    assert_eq!(
        greeter["members"][2]["doc"],
        serde_json::json!({
            "short": "Greets `$who`, and remembers the name.",
            "see": [r"Core\Str::length"],
            "example": ["examples/greeting.nvs"],
        })
    );

    // The static member is documented with prose and no tag, so it carries a
    // `short` and neither list — an empty array would read as "documented with
    // no targets" rather than "no tag written".
    assert_eq!(
        greeter["members"][3]["doc"],
        serde_json::json!({ "short": "Greets nobody in particular." })
    );

    // The constructor carries no `///` at all, so it carries no card.
    assert!(greeter["members"][1]["doc"].is_null());

    let volume = declared(&program, "enums", "Volume");
    assert_eq!(
        volume["doc"],
        serde_json::json!({ "short": "How loudly to greet." })
    );
    assert_eq!(
        volume["cases"][0]["doc"],
        serde_json::json!({ "short": "Barely audible." })
    );
}

/// A class names its parent under `extends` and its interfaces under
/// `implements`, and an interface names the interfaces it extends under an
/// `extends` list. Every name is fully qualified: the fixture writes `Item` and
/// `Named` by their short names inside `namespace Shop;`, and the document says
/// `Shop\Item` and `Shop\Named`. A declaration with no lineage carries neither
/// key, by the registry's omission rule.
#[test]
fn a_user_class_names_its_parent_and_interfaces_fully_qualified() {
    let lineage = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/meta/lineage.nvs")
        .display()
        .to_string();
    let program = program(&["--json", &lineage]);

    let post = declared(&program, "classes", r"Shop\Post");
    assert_eq!(post["extends"], r"Shop\Item");
    assert_eq!(post["implements"], serde_json::json!([r"Shop\Named"]));

    let named = declared(&program, "interfaces", r"Shop\Named");
    assert_eq!(named["extends"], serde_json::json!([r"Shop\Printable"]));

    let item = declared(&program, "classes", r"Shop\Item");
    assert!(item.get("extends").is_none(), "`extends` was emitted empty");
    assert!(
        item.get("implements").is_none(),
        "`implements` was emitted empty"
    );
    let printable = declared(&program, "interfaces", r"Shop\Printable");
    assert!(
        printable.get("extends").is_none(),
        "`extends` was emitted empty"
    );
}

/// The registry document's rosters, and the one this adds beside them.
fn roster(document: &serde_json::Value, key: &str) -> Vec<serde_json::Value> {
    document[key]
        .as_array()
        .unwrap_or_else(|| panic!("`{key}` is an array"))
        .clone()
}

/// `capabilities` is a top-level roster like the four beside it, and it carries
/// what `nvs_stdlib::registry::CAPABILITIES` declares: a class, a member, and
/// the capability only where the row names one. A row naming none has no
/// `capability` key rather than a `null` one, which is `rule:tooling/meta-json`'s
/// omission rule and the only spelling of "needs nothing" a consumer can tell
/// from "not written".
#[test]
fn the_meta_document_carries_a_capabilities_roster_beside_its_other_four() {
    let (doc, ok) = meta(&["--json"]);
    assert!(ok, "the registry dump succeeds");
    let document: serde_json::Value = serde_json::from_str(&doc).expect("the document is JSON");

    for key in ["exceptions", "interfaces", "attributes", "directives"] {
        assert!(!roster(&document, key).is_empty(), "`{key}` is populated");
    }
    let capabilities = roster(&document, "capabilities");
    assert!(
        !capabilities.is_empty(),
        "`capabilities` is populated beside them"
    );

    let row = |class: &str, member: &str| {
        capabilities
            .iter()
            .find(|r| r["class"] == class && r["member"] == member)
            .unwrap_or_else(|| panic!("{class}::{member} has a row"))
            .clone()
    };

    // A gated member names the capability in the spelling `nvs.toml` grants it
    // under, so the string is pasteable into a configuration file as it stands.
    assert_eq!(
        row(r"Core\IO", "read"),
        serde_json::json!({ "class": r"Core\IO", "member": "read", "capability": "fs.read" })
    );
    // An ungated member is the same row without the key.
    assert_eq!(
        row(r"Core\IO", "stdin"),
        serde_json::json!({ "class": r"Core\IO", "member": "stdin" })
    );
}

/// Every row of the roster resolves inside the same document, so a consumer can
/// join `(class, member)` onto a member's card without reaching for anything
/// this command did not print. `nvs-stdlib`'s completeness test holds the table
/// against the registry; this holds the *document* against itself, which is what
/// a consumer outside the binary actually has.
#[test]
fn every_capabilities_row_names_a_class_and_member_the_registry_holds() {
    let (doc, ok) = meta(&["--json"]);
    assert!(ok, "the registry dump succeeds");
    let document: serde_json::Value = serde_json::from_str(&doc).expect("the document is JSON");

    for row in roster(&document, "capabilities") {
        let class = row["class"].as_str().expect("a row names a class");
        let name = row["member"].as_str().expect("a row names a member");
        // `member` panics with the pair's own spelling when either half misses,
        // which is the message this case exists to produce.
        let found = member(&document, class, name);
        assert_eq!(
            found["name"], name,
            "{class}::{name} resolves in the document"
        );
    }
}

/// The join is at render time and nothing is pushed onto a member row:
/// `rule:security/capability-declaration-is-one-table`'s second paragraph
/// rejects a per-member field, and a field appearing here later would be that
/// shape arriving through the document instead of through the table.
#[test]
fn no_member_row_gained_a_capability_field() {
    let (doc, ok) = meta(&["--json"]);
    assert!(ok, "the registry dump succeeds");
    let document: serde_json::Value = serde_json::from_str(&doc).expect("the document is JSON");

    for class in roster(&document, "classes") {
        let name = class["name"].as_str().expect("a class names itself");
        for member in class["members"].as_array().into_iter().flatten() {
            assert!(
                member["capability"].is_null(),
                "{name}::{} carries no capability field",
                member["name"]
            );
        }
    }
}

/// The attack written against this command: doc comments holding quotes, a
/// backslash, `?>`, a tab, a right-to-left mark and text shaped like JSON. The
/// document is one valid JSON value, and each prose line comes back exactly as
/// the `///` run wrote it.
// covers: tools:cli/nvs-meta-json
#[test]
fn meta_json_carries_doc_comments_that_json_must_escape_unchanged() {
    let attack = nvs_repo::path("tests")
        .join("hostile")
        .join("tools")
        .join("cli")
        .join("nvs-meta-json")
        .join("01-doc-comments-that-are-hard-to-write-as-json.nvs");
    let program = program(&["--json", &attack.display().to_string()]);

    let hard = declared(&program, "classes", "Hard");
    assert_eq!(
        hard["doc"]["short"],
        r#"1. Quotes "like this", a backslash \ and a closing tag ?> in the text."#
    );
    assert_eq!(hard["doc"]["see"], serde_json::json!(["Hard::quoted"]));
    assert_eq!(
        hard["members"][0]["doc"]["short"],
        "2. Text that is not ASCII: café, 日本, 😀, and a right-to-left mark \u{200f} here."
    );
    assert_eq!(
        hard["members"][1]["doc"]["short"],
        r#"3. Text that looks like JSON: {"key": [1, 2]}, and `*/` that does not end anything."#
    );
    assert_eq!(
        hard["members"][1]["signature"],
        r#"quoted(string $text = "\"\\\n"): string"#
    );

    let mode = declared(&program, "enums", "Mode");
    assert_eq!(
        mode["cases"][0]["doc"]["short"],
        "The first case.\tA tab is before this sentence."
    );
}
