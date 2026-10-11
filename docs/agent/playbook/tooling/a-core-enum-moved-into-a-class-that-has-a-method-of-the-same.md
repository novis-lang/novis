- **A `Core` enum moved into a class that has a method of the same word fails `nvs-cli`'s
  `no_two_index_lines_open_with_the_same_symbol`, though the program compiles.** `Core\Xml\Node::kind`
  and `Core\Xml\Node::Kind` were one line to a check that lowercased every symbol, because `nvs agent
  show` used to match without case. `show` now takes the exact symbol first
  (`crates/nvs-cli/src/agent.rs:@show_target`), so a case-only pair is fine. If that test fails on a
  rename again, the pair is an exact duplicate, which is a real clash. `until: gone crates/nvs-cli/tests/agent.rs:two_symbols_that_differ_only_by_case_each_show_their_own_card`
