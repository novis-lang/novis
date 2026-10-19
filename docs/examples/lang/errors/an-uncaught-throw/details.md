The report goes to the error output, and the normal output does not get it. By default the report is
one line of JSON. `msg` is the message, `fields.class` is the error class, and `nodes` has one entry
for each function with `function`, `file` and `line`. The last entry is `<script>`, the top level of
the file.

With `format = "text"` in the `[log]` table of `nvs.toml`, the report is text. The first line is
`[error]` and the message. The next line is the class. Then each function gets one line, such as
`#0 Repo::get() at main.nvs:4`.

The report has the class of your own error, such as `BadInput`, when you throw one.

The report does not have the error in `previous`. To see the cause, catch the error and print
`$e->previous?->message` yourself.

Before the program ends, every `finally` block that the error leaves still runs. What it prints
stays in the normal output.

The exit status is 1 for every uncaught throw.
