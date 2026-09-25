- **A test that writes one log record twice gets one line, and the second write leaves the buffer
  empty.** `Core\Log::write` goes through `nvs_runtime::floor::admit_log_record`'s table
  (`rule:errors/a-repeat-is-bounded-at-the-sink-that-suffers`), whose identity clears `ts`,
  `request_id`, `trace_id` and `span_id`, so two writes differing only in the request or the trace
  are one record and the failure reads as serde's `EOF while parsing a value`. Give each write its
  own message or `source`, and reach for `floor::expire_log_windows` only where the *count* is the
  subject. [until: gone crates/nvs-stdlib/src/log.rs:admit_log_record]
