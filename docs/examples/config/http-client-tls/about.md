Settings for every outbound `https` call: which certificates are trusted, the oldest TLS version
that is allowed, and a debug file for session secrets.

`roots` is the list of trusted certificate authorities. When it is not set, Novis uses the Mozilla
set that ships with it. You can add your own authority to that set, or list only your own files.
`min_version` is `1.2` or `1.3`, and the default is `1.2`. `keylog` is a file that Novis writes
session secrets to, so you can read your own traffic in a protocol analyser. That file can decrypt
everything the host sends, so `keylog` is not allowed on a production host.

Only the person who runs the server sets these keys. All requests use one HTTP client, so a program
cannot change them.

**Good to know:** a running server applies a change at the next reload, and new connections use
it. If a file in `roots` contains no certificate, the old settings stay in use. The reload then
reports the key as not applied.
