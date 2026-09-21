```
1. longest match:  host mounts by prefix  ->  host-less mounts by prefix  ->  404
2. strip the prefix
3. [server] static  &&  the remainder is an existing non-.nvs file under the mount root  -> serve it
4. dispatch == "path"  &&  the remainder is an existing .nvs file under the mount root   -> run it
5. otherwise                                                                             -> run the mount's entry
```

In production — `dispatch = "entry"`, `static = false` — steps 3 and 4 do not run: match, strip, run the entry. In development the sequence is `try_files $uri /index.nvs`, the pattern every PHP application already deploys under. Both directives are `Boot`-class startup defaults a mode selects and no request may flip (`rule:config/a-startup-default-is-never-flipped`), because a flip would appear to change `dispatch` for a request already dispatched.

**No step takes its case rule from the filesystem.** A prefix is matched exactly, and in steps 3 and 4 a remainder that differs from the file on disk only in case is a file that is not there: `/STYLE.CSS` does not serve `style.css` on Windows or macOS, because it would not on Linux. It is `rule:programs/path-case`'s comparison, at the same cost — the canonical path is already in hand. The host is the one part that folds (`rule:http-server/host-matching-is-on-the-host-part-only`), because its specification says it does.

`[server] static` is a boolean and the *path* is each mount's own root, which is what makes assets work for a fleet of modules rather than for one. What serving a file means in either mode is `rule:http-server/static-serving-is-one-policy`. A URL that maps to a file in production too — `php -S` in both modes — is rejected: production has a front controller and a compiled route table, and every extra URL-to-file resolution is surface on the hot path bought for a scratch workflow.
