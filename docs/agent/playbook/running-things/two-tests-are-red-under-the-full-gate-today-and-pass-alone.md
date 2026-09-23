- **Two tests are red under the full gate today and pass alone, and neither is yours.**
  `nvs_host`'s `a_race_past_a_dead_address_answers_on_the_next_one` takes its dead address from a
  listener it dropped, so a binary beside it can take that freed port and answer the race;
  `nvs_server`'s `a_disconnected_clients_isolates_are_left_behind_on_no_core` reads a response load
  turns into a reset. Read the "passed alone" verdict before hunting what you broke, and fix either
  in a commit of its own.
  [until: gone crates/nvs-host/src/net.rs:a_race_past_a_dead_address_answers_on_the_next_one]
