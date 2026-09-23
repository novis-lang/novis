- **A `Node::Object` is spelled `{"$class":…,"$id":…,"$properties":{…}}` in JSON, so a producer whose
  wire shape has to be a plain object cannot use one.** `nvs_render::json`'s rule is that a kind JSON
  has no value for becomes a `$`-tagged one-key object, and `Node::Object` means *class instance*
  (`crates/nvs-render/src/lib.rs:340`), which a stack frame is not — a frame is `Node::Frame`, the one
  untagged exception, for exactly that reason. Read what each rendering writes before picking a node
  kind for a new producer: `grep -n "Node::Span" crates/nvs-render/src` finds every exhaustive match,
  and there are only the two. [until: gone crates/nvs-render/src/json.rs:AsNode]
