A `TRACE`-flagged run exports its `call`, `gc`, `spawn` and `query` events as speedscope's
evented-profile JSON — open/close pairs at a timestamp, which is exactly what the trace already holds.
speedscope.app opens the file as one scrollable timeline with all four kinds on it, and a spawned
child's events nested under its parent's `spawn` bar.

This is an **additional** export, not a replacement: Callgrind for the aggregate profile, Clover and
lcov for coverage, and Novis-native NDJSON for the raw trace stay as `rule:testing/debug-surface`
lists them. It reuses the one open format the sampling profiler already commits to rather than
adding a second timeline format with a bespoke viewer to build and maintain.

The NDJSON trace remains the format no third-party tool reads; this export closes that gap for
visualisation, and for nothing else. The exact CLI flag that selects it is left to the exporter's
implementation, the same way the Clover and lcov shapes were.
