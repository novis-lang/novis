- **A lowering arm that hands back its own operand owes a retain, and skipping it is a heap
  corruption that passes `nvs.exe run` and fails only under the conformance runner.** The exit
  status is `-1073740940` (`0xC0000374`, `STATUS_HEAP_CORRUPTION`) with no message: a double release
  only trips the allocator once the process does enough afterwards. `Lowering::convert`'s free `from
  == to` row carries the rule, so a new arm returning its operand repeats its
  `aliasing_read`/`emit_retain` pair; do not copy `lower_class_reference`, which owes none because a
  descriptor is immortal. [until: reviewed 2026-09-06]
