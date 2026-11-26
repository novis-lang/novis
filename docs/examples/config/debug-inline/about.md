Decides whether a debugging dump also appears in the page, or only in the log.

`Core\Debug::dump` always writes its value to the log. With this setting on, a dump made while an
HTML page is built is also added to the end of that page. A JSON response is never changed.

The default depends on the mode in `[mode] default`. In `development` the dumps are added to the
page, and in `production` they are not. A program cannot turn this setting on. In production, a
dump that you forgot in the code is only a line in the log. A program can turn it off for its own
request with `Core\Config::set('debug.inline', 'false')`.

`[debug] store` records each request for the debug viewer, a page in your browser. When it is also
on, the block in the page starts with the number of queries the request ran. It also has a link
that opens this request in the debug viewer.

**Good to know:** on the command line, a dump always goes to standard error. This setting has no
effect there.
