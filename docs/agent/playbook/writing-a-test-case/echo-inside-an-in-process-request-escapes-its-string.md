- **`echo` inside an in-process request escapes its string**, so a `--EXPECT--` quoting a throw's
  message sees `&quot;` and `&#39;`. Write the message with no `"` and no `'` in it, or expect the
  entities. [until: gone crates/nvs-stdlib/src/html.rs:&#39;]
