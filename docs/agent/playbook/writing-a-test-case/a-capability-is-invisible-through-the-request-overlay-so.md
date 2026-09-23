- **A capability is invisible through the request overlay, so narrowing one via `Core\Config::set`
  pins nothing.** `capability::refusal` reads `config.snapshot().config.capabilities`, never the
  `Request` overlay, and `Request::set` refuses the block anyway: a grant is a list, with no
  quantity to compare. Narrow a capability in the snapshot, as
  `a_child_cannot_widen_a_capability_its_parent_narrowed` does.
  [until: gone crates/nvs-config/src/directive.rs:"capabilities", class: Class::RuntimeTighten]
