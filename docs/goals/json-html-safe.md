---
looper: 1
title: JSON text that Novis writes is safe inside an HTML page
summary: "Reached when every JSON text Novis writes from a program's values escapes <, >, &, ', U+2028 and U+2029 as \\u escapes, so Core\\Json::encode's and encodeEach's output cannot open or close a tag, end a single-quoted attribute or end a script line wherever a program writes it, with no option to turn it off, and the rule, the cards, the examples and an attack say so."
context: [crates/nvs-stdlib/src/json.rs, crates/nvs-stdlib/src/json/pieces.rs, docs/rules/core-classes/json-whole-and-stream.md, docs/reference/core/Json.md]
stages:
  - id: 1
    title: "the encoder escapes the six characters as it writes each string, and the rule says why"
    status: open
    context: [crates/nvs-stdlib/src/response.rs, crates/nvs-stdlib/src/jwt.rs, docs/rules/security/response-body-is-one-typed-member.md]
  - id: 2
    title: "every test, example and page that shows encoded JSON with those characters is updated, and an attack pins it"
    status: open
    context: [docs/examples/README.md, tests/hostile/README.md, docs/rules/testing/feature-proofs.md]
  - id: 3
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: json-html-escape
    stage: 1
    name: encode and encodeEach escape the six characters in strings and keys, and decode reads them back
    run: [cargo, test, --lib, "json::"]
    timeout: 1800
    tests:
      - encode_escapes_angle_brackets_and_ampersand_in_strings_and_keys
      - encode_escapes_the_two_javascript_line_separators
      - encode_escapes_the_apostrophe_for_a_single_quoted_attribute
      - encode_each_pieces_escape_what_encode_escapes
      - an_escaped_text_decodes_to_the_value_it_was_made_from
      - pretty_and_escape_unicode_output_escape_the_same_characters
---

# JSON text that Novis writes is safe inside an HTML page

## Why

`Core\Json::encode` writes `<`, `>` and `&` as they are, and its result is a plain `string`. So
`Core\Response::stream('text/html')->write(Core\Json::encode($value))` compiles and writes a user's
`<script>` into the page unescaped. A JSON value placed in an inline `<script>` block can close it the
same way. The goal `queue-id-and-json-stream` gives `Core\Json::encodeEach`'s pieces the taint that
`encode`'s text has, which makes its pieces reach the same raw writes. This goal runs first, so that
change is safe when it lands.

## Standing decisions

1. **Escape, do not taint (the user's call, 2026-11-06).** Every string and key the encoder writes has
   `<` as `\u003c`, `>` as `\u003e`, `&` as `\u0026`, `'` as `\u0027`, U+2028 as `\u2028` and U+2029 as `\u2029`. The
   text stays valid JSON and every decoder reads back the same value. Considered and declined:
   making `encode`'s result tainted when its input is. An API that returns user data as JSON is the
   common, correct case, and it would need a launder step for nothing. Taint also cannot tell a JSON
   response from an HTML page, and the escaped text is safe in both.
2. **The apostrophe too (the user's call, 2026-11-06).** A page often puts JSON in a single-quoted
   attribute, such as `data-config='{"page": 2}'`, because JSON's own `"` breaks a double-quoted one.
   With `'` and `&` escaped, a value cannot end that attribute. It costs five bytes per apostrophe,
   which is more common in text than the other four. Go does not escape it, and PHP has the flag
   `JSON_HEX_APOS`.
3. **No option to turn it off.** Go's encoder escapes the same way by default. PHP has it as the flag
   `JSON_HEX_TAG`, which is the kind of option a program forgets. A program that needs the raw bytes is
   a later decision, recorded under Declined in the rule, not an option now.
4. **One place.** The escape is a wrapper around `serde_json`'s `Formatter` that overrides
   `write_string_fragment`, so it applies to the compact and the pretty writer, to keys and values,
   and to `encodeEach`'s pieces. It is not a second pass over the output, as `escape_non_ascii` is for
   `{ escapeUnicode: true }`. `crates/nvs-stdlib/src/json.rs:394`, `crates/nvs-stdlib/src/json.rs:1106`,
   `crates/nvs-stdlib/src/json.rs:1962`
5. **Every writer of a program's values.** Stage 1 finds every place Novis writes JSON from a program's
   values (`Core\Response::json` if it exists, `Core\Http\Client`'s `json` option, `Core\Jwt`, the log's
   JSON lines, `nvs` output a program's values reach) and routes each through the same formatter, or
   records in the rule why it cannot reach an HTML page.
6. **What this spends.** Five bytes more for each escaped character, the apostrophe included, and nothing for text without them.
   The formatter already checks every byte of a string for quotes and control characters, so the six
   are more arms of the same check. The `Core\Json::encode` bench records the cost.

## Stages

### 1. the encoder escapes the six characters as it writes each string, and the rule says why

- [ ] The new rule `security/json-text-is-safe-inside-html`, with decisions 1 to 3 and the Declined
  list, and `rule:core-classes/json-whole-and-stream` cites it.
  `docs/rules/core-classes/json-whole-and-stream.md:1`
- [ ] The formatter wrapper of decision 4, used by every `Sink`. `crates/nvs-stdlib/src/json.rs:1106`
- [ ] Every other JSON writer of decision 5 found and routed, or named in the rule.
  `crates/nvs-stdlib/src/jwt.rs:1`
- [ ] The six tests of the `json-html-escape` check, each run once by name.
- [ ] The cards of `encode`, `encodeEach` and `Core\Json\Pieces` say the output is safe inside HTML.
  `crates/nvs-stdlib/src/json.rs:587`

### 2. every test, example and page that shows encoded JSON with those characters is updated, and an attack pins it

- [ ] Every `.out`, `.nvst` and `.lspt` expectation that freezes `encode`'s output with `<`, `>` or `&`
  in a string, found by search, updated. List them in the handoff.
- [ ] `docs/reference/core/Json.md` and the `encode` about page say it in one sentence each.
  `docs/reference/core/Json.md:6`, `docs/examples/core/Json/encode/about.md:1`
- [ ] An attack: a request puts `</script><script>` in a value, the program writes `encode`'s result
  into an HTML page through `Core\Response::stream`, and the page has no second script.
  `tests/hostile/core/Json/encode/`
- [ ] `bun nv proofs --id 'Core\Json::encode'` and `--id 'Core\Json::encodeEach'` owe nothing, and
  `bun nv proofs --comments` over the changed `.nvs` files passes.

### 3. the goal is verified once, and every failure is fixed

- [ ] `bun nv affected --run`, every failure fixed in one pass, then once more.
- [ ] The goal-end gates of AGENTS.md § *In the loop*.

## Handoff

### State

Written 2026-11-06 from a review of `queue-id-and-json-stream`'s Backlog. Nothing is built yet.

### Next

Stage 1, from its first item.

### Backlog

Nothing yet.
