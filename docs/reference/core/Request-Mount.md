---
summary: the door a request came through — the prefix the server stripped and the glob captures of the mount row that took it, so one compiled program serves many tenants
keywords: SCRIPT_NAME, PATH_INFO, base path, base URL, RewriteBase, subdirectory install, sub-folder deployment, virtual host, multi-tenant, subdomain routing, $_SERVER['REQUEST_URI']
---

`Core\Request\Mount` is what `Core\Request::mount()` answers with, and it holds two facts about the door
this request arrived at: `prefix()` is what the server took off the front of the path before
`Core\Request::path()` answered it, and `captures()` are the glob captures of the mount row that took the
request, in order — `{1}` is `captures[0]`. One member answers the pair because a request holding one
mount's prefix and another mount's captures is a bug the shape rules out.

**It is never `null`**, where `Core\Request::route()` is. Every request that reached a program reached it
*through* a mount, so a door that strips nothing and holds no glob answers `""` and an empty array rather
than an absence — there is no case to write for "served without a mount". With no request in front of it
at all, naming it refuses the way every reader of `Core\Request` does.

**A mount says where a request arrives and which file answers it, and nothing else.** Its key set is
closed — `prefix`, `host`, `scan`, `entry`, `origin` — and carries no mode, no limits and no capabilities;
what the code answering the request *may do* is the `[[app]]` block's, keyed on the entry file path. The
two usually cover the same tree, and reading one for the other is the mistake this split exists to
prevent: what you learn here is routing, never policy.

**The table expands at boot, not per request.** A `scan` glob is resolved against the disk once at
startup — and again at each configuration reload, in development also under hot reload's revalidation — into
ordinary mounts whose paths were each checked to lie inside `[server] root`. So a prefix reaching a
program is a row an operator wrote, and no path is ever derived from a URL at request time.

That is why the prefix is plain text and **the captures are `tainted`**: which row answers is the peer's
choice, and a capture is exactly what a multi-tenant program keys its data by. A capture reaching a query
or a path launders the way anything else off a request does. Nothing here is cached — both facts live on
the inbound request and this class is a reading of them — so a read costs one instance and one array of
the mount's own captures, a handful of values against a glob's one or two.

```nvs skip
<?nvs
// Served by a mount that scans `*/public/index.nvs` under `[server] root` and
// mounts each match at `/{1}`, so `/acme/orders` arrives here as `/orders`.
Core\Request\Mount $mount = Core\Request::mount();
tainted string $tenant = $mount->captures()[0];

echo "serving ", $tenant, " under ", $mount->prefix(), "\n";
```

A program therefore never derives its own base path from the request target, and `Core\Router::url`
prepends the prefix on the way back out, so the same compiled route table serves at `/ModuleA`, at
`/{1}` or at `/` with no recompile. `$_SERVER['SCRIPT_NAME']`, `PATH_INFO` and the `RewriteBase` guessing
that goes with them have nothing left to describe. Without a request, the class is unreachable in the
ordinary way:

```nvs
<?nvs
try {
    Core\Request::mount();
    echo "not reached\n";
} catch (LogicError $none) {
    echo "nothing was mounted here\n";
}
```
```output
nothing was mounted here
```
