---
# GENERATED FILE — written by website/scripts/sync-rules.mjs from docs/rules/. Do not edit.
title: "Running as a service"
description: "An operator installs a service from the command line. The installer fails closed, and nothing about it is on the request path."
editUrl: false
lastUpdated: false
tableOfContents: false
prev:
  link: /docs/rules/packaging/extensions/
  label: "Extensions"
next:
  link: /docs/rules/php-migration/
  label: "Migrating from PHP"
---

<p class="nv-section-lead">An operator installs a service from the command line. The installer fails closed, and nothing about it is on the request path.</p>

<div class="nv-counts"><div class="nv-count" data-kind="total"><span class="nv-count-value">10</span><span class="nv-count-label">rules</span></div><div class="nv-count" data-kind="shipped"><span class="nv-count-value">6</span><span class="nv-count-label">shipped</span></div><div class="nv-count" data-kind="designed"><span class="nv-count-value">4</span><span class="nv-count-label">designed</span></div><div class="nv-count" data-kind="php"><span class="nv-count-value">7</span><span class="nv-count-label">differ from PHP</span></div></div>

<ol class="nv-rule-list"><li><a href="#pdf-decoding-ships-in-the-image-package">PDF is a decode-only format of <code>nvs/image</code>, in the second wave, and never a member of <code>nvs/pdf</code></a><span class="nv-rule-list-status" data-status="designed">Designed</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#a-service-is-one-stored-argv"><code>nvs service</code> is a namespace, and everything after a mandatory <code>--</code> is stored verbatim as the argv the service runs</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#the-installer-is-a-sink">The service installer fails closed: a closed <code>serve</code>/<code>run</code> allowlist, no relative path, no argv without <code>--config</code>, no install whose output goes nowhere, no password on a command line</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#the-argv-lives-in-imagepath">On Windows the argv is encoded into the SCM's one <code>ImagePath</code> string, the binary path is always quoted, and nothing else holds it</a><span class="nv-rule-list-status" data-status="designed">Designed</span></li><li><a href="#a-service-runs-as-a-virtual-account">A service's default identity is the per-service virtual account <code>NT SERVICE\&lt;name&gt;</code>, and <code>LocalSystem</code> is never the default</a><span class="nv-rule-list-status" data-status="designed">Designed</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#a-service-answers-its-manager">A stop drains, a <code>PARAMCHANGE</code> reloads, and lifecycle records go to the event log beside the configured log destination</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#the-unit-is-printed-and-install-is-the-opt-in">On Linux <code>nvs service unit</code> prints the systemd unit and touches nothing; writing it is an explicit <code>--install</code></a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#the-generated-unit-is-hardened">The generated unit carries the hardening block, <code>Type=notify</code>, <code>MemoryMax</code> from <code>[limits]</code>, and <code>AmbientCapabilities</code> only for a privileged port</a><span class="nv-rule-list-status" data-status="designed">Designed</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#a-bundle-may-not-install-itself">A single-file bundle may not install itself as a service</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#a-service-is-operator-surface">There is no <code>Core\Service</code>: only an operator installs a service, from the command line, and nothing about it is on the request path</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li></ol>

<div class="nv-rule" id="pdf-decoding-ships-in-the-image-package">

## PDF is a decode-only format of `nvs/image`, in the second wave, and never a member of `nvs/pdf`

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#pdf-decoding-ships-in-the-image-package"><code>packaging/pdf-decoding-ships-in-the-image-package</code></a>
</div>

PDF joins the image component's format roster **decode only**, in the second wave beside SVG, with
`Format` gaining a `Pdf` case and no new entry point: the plan/terminal shape and the component's closed
set of exports are unchanged ([`core-classes/image-format-roster`](/docs/rules/core-classes/uris-and-images/#image-format-roster "The format roster is a closed table of decoders and encoders, each naming what implements it"),
[`core-classes/pdf-page-is-an-image-source`](/docs/rules/core-classes/pdf-and-spreadsheets/#pdf-page-is-an-image-source "A PDF page is a decode-only format of the image component: one page per open, priced by the pixel cap")).

It is not a member of `nvs/pdf`. A writer and an interpreter share no code, and a raster produced in the
generation package would have to recross the boundary to enter the image pipeline that is the whole
point of loading one. Generation stays [`core-classes/pdf-render-has-no-io`](/docs/rules/core-classes/pdf-and-spreadsheets/#pdf-render-has-no-io "PDF generation is a first-party extension that performs no I/O, and an unresolved asset throws")'s; text extraction,
page manipulation and forms are different jobs and stay in the third-party channel.

The job this replaces is PHP's ImageMagick-delegating-to-Ghostscript pair — an installed, unsandboxed
interpreter with an RCE history long enough that ImageMagick's stock policy ships with the PDF coder
disabled. A PDF interpreter is a strictly larger hostile-bytes case than any format already on the
roster, and the sandbox is where a parser that size belongs.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>There is no Ghostscript to install and no ImageMagick policy to loosen; a page becomes an image through the same sandboxed component every other format uses</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-classes/pdf-and-spreadsheets/#pdf-page-is-an-image-source" title="A PDF page is a decode-only format of the image component: one page per open, priced by the pixel cap"><code>core-classes/pdf-page-is-an-image-source</code></a> <a href="/docs/rules/core-classes/uris-and-images/#image-format-roster" title="The format roster is a closed table of decoders and encoders, each naming what implements it"><code>core-classes/image-format-roster</code></a> <a href="/docs/rules/core-classes/pdf-and-spreadsheets/#pdf-render-has-no-io" title="PDF generation is a first-party extension that performs no I/O, and an unresolved asset throws"><code>core-classes/pdf-render-has-no-io</code></a> <a href="/docs/rules/packaging/extensions/#an-extension-package-carries-two-payloads" title="An extension package may carry Novis source beside its .nvsx, under one namespace, and the component's manifest registers exactly one class"><code>packaging/an-extension-package-carries-two-payloads</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0128.md">record 0128</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0120.md">record 0120</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0121.md">record 0121</a></dd></div></dl>

</div>

<div class="nv-rule" id="a-service-is-one-stored-argv">

## `nvs service` is a namespace, and everything after a mandatory `--` is stored verbatim as the argv the service runs

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#a-service-is-one-stored-argv"><code>packaging/a-service-is-one-stored-argv</code></a>
</div>

```
nvs service install   <name> [options] -- <verbatim nvs args…>
nvs service uninstall <name>
nvs service start | stop | status <name>
nvs service run       <name>                                  # the service manager's entry point
nvs service unit      <name> [options] -- <verbatim nvs args…>  # Linux: print, install nothing
```

A service is this binary, registered with the platform's service manager, running **one stored argv**.
Everything left of `--` belongs to the installer; everything right of it is stored untouched and never
interpreted, which is what makes every parameter `nvs` accepts passable. `--` is mandatory: without it,
`--start` is ambiguous between the installer and the hosted program, a defect `mysqld --install` has
and Novis does not inherit. `nvs install-service` is accepted as a hidden alias for the muscle memory
`mysqld --install` and `httpd -k install` built.

The verbs are namespaced like `nvs ctl` because they act on a server rather than on files, and the name
is positional and is the same identity `nvs ctl --socket` uses. `start`/`stop`/`status` are thin — the
SCM directly on Windows, `systemctl` by argv with no shell on Linux
([`core-classes/process-is-argv-only`](/docs/rules/core-classes/processes-and-files/#process-is-argv-only "Core\Process is the one way to run another program, and there is no shell string anywhere in it")) — and earn their second spelling by reporting what no
service manager knows: the in-flight request count, and drain progress during a stop, asked over the
control socket ([`config/one-local-control-socket`](/docs/rules/config/reloading-and-control/#one-local-control-socket "The server is controlled over one local socket whose owner and mode are the authentication, and nvs ctl is its client")).

What that argv may name is [`packaging/the-installer-is-a-sink`](/docs/rules/packaging/running-as-a-service/#the-installer-is-a-sink "The service installer fails closed: a closed serve/run allowlist, no relative path, no argv without --config, no install whose output goes nowhere, no password on a command line")'s closed list.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>There is no shim — no NSSM, no WinSW, no hand-written unit around <code>php-fpm</code> — and the hosted program's own flags follow a <code>--</code> the installer never reads past</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/packaging/running-as-a-service/#the-installer-is-a-sink" title="The service installer fails closed: a closed serve/run allowlist, no relative path, no argv without --config, no install whose output goes nowhere, no password on a command line"><code>packaging/the-installer-is-a-sink</code></a> <a href="/docs/rules/packaging/running-as-a-service/#the-argv-lives-in-imagepath" title="On Windows the argv is encoded into the SCM's one ImagePath string, the binary path is always quoted, and nothing else holds it"><code>packaging/the-argv-lives-in-imagepath</code></a> <a href="/docs/rules/packaging/running-as-a-service/#the-unit-is-printed-and-install-is-the-opt-in" title="On Linux nvs service unit prints the systemd unit and touches nothing; writing it is an explicit --install"><code>packaging/the-unit-is-printed-and-install-is-the-opt-in</code></a> <a href="/docs/rules/config/reloading-and-control/#one-local-control-socket" title="The server is controlled over one local socket whose owner and mode are the authentication, and nvs ctl is its client"><code>config/one-local-control-socket</code></a> <a href="/docs/rules/core-classes/processes-and-files/#process-is-argv-only" title="Core\Process is the one way to run another program, and there is no shell string anywhere in it"><code>core-classes/process-is-argv-only</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0093.md">record 0093</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0078.md">record 0078</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-cli/src/service.rs"><code>crates/nvs-cli/src/service.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-cli/src/ctl.rs"><code>crates/nvs-cli/src/ctl.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="the-installer-is-a-sink">

## The service installer fails closed: a closed `serve`/`run` allowlist, no relative path, no argv without `--config`, no install whose output goes nowhere, no password on a command line

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#the-installer-is-a-sink"><code>packaging/the-installer-is-a-sink</code></a>
</div>

The trailing argv is the sharpest sink in the project ([`security/sink-predicate`](/docs/rules/security/tainted-data/#sink-predicate "A string or bytes parameter is a sink when its content becomes an instruction a parser executes")): the command
runs elevated, and what it stores is executed by a privileged account at every boot until somebody
removes it. So the default is refusal, and the allowlist is closed:

| Refused | Because |
|---|---|
| A subcommand other than `serve` or `run` | everything else exits at once — a crash loop, forever — or needs a terminal |
| `--fault-inject`, on any subcommand | a hook that must never be reachable from a served request, now with a privileged account |
| Any relative path, in the argv or an installer option | a Windows service starts in `System32`: a first-boot failure as an opaque SCM code |
| An argv with no `--config` | it would fall back to `./nvs.toml` ([`config/the-root-is-config-else-nvs-toml-else-the-shipped-defaults`](/docs/rules/config/the-file-and-the-tree/#the-root-is-config-else-nvs-toml-else-the-shipped-defaults "The root of the tree is every --config in order, else ./nvs.toml, else the shipped defaults")), making the configuration a property of the starting directory; a service names it absolutely |
| Neither `--log-file` nor a `[log]` file or syslog destination | a service has no console handle, so stderr goes nowhere and a refused compile leaves no trace; `stderr` is not a destination |
| An `--account` password on the command line | readable by other users; it is prompted, and is `secret` for its whole life ([`security/secret-qualifier`](/docs/rules/security/secrets/#secret-qualifier "secret is a second, independent compile-time qualifier, written before tainted and in that order alone")) |
| Running from a bundle | [`packaging/a-bundle-may-not-install-itself`](/docs/rules/packaging/running-as-a-service/#a-bundle-may-not-install-itself "A single-file bundle may not install itself as a service") |

Every surviving path is canonicalized and stored absolute. Each refusal is an `E0630`–`E0634`
diagnostic naming what was refused and why — two pairs of rows share a code because they share a
reason — never a bare non-zero exit. The refusals run in front of `nvs service unit` too, so an
operator learns what would have been refused without an elevated shell and without installing
anything.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p><code>sc create</code> and a hand-written unit store whatever they are given; the installer refuses each of these by name with an <code>E063x</code> diagnostic and installs nothing</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/security/tainted-data/#sink-predicate" title="A string or bytes parameter is a sink when its content becomes an instruction a parser executes"><code>security/sink-predicate</code></a> <a href="/docs/rules/config/the-file-and-the-tree/#the-root-is-config-else-nvs-toml-else-the-shipped-defaults" title="The root of the tree is every --config in order, else ./nvs.toml, else the shipped defaults"><code>config/the-root-is-config-else-nvs-toml-else-the-shipped-defaults</code></a> <a href="/docs/rules/security/secrets/#secret-qualifier" title="secret is a second, independent compile-time qualifier, written before tainted and in that order alone"><code>security/secret-qualifier</code></a> <a href="/docs/rules/packaging/running-as-a-service/#a-bundle-may-not-install-itself" title="A single-file bundle may not install itself as a service"><code>packaging/a-bundle-may-not-install-itself</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0093.md">record 0093</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0088.md">record 0088</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0103.md">record 0103</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0048.md">record 0048</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-cli/src/service.rs"><code>crates/nvs-cli/src/service.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="the-argv-lives-in-imagepath">

## On Windows the argv is encoded into the SCM's one `ImagePath` string, the binary path is always quoted, and nothing else holds it

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<a class="nv-rule-id" href="#the-argv-lives-in-imagepath"><code>packaging/the-argv-lives-in-imagepath</code></a>
</div>

The Windows SCM stores **one string**, and the process gets it back through `CommandLineToArgvW`. The
trailing argv is encoded into that string under those rules — the backslash-run-before-a-quote rule
included — and **that string is the only record of what the service runs**. `sc qc <name>` shows an
auditor literally what runs, with no second place to look.

A sidecar argv file with a short `ImagePath` would make the round trip exact and is refused anyway: a
file that decides what a `LocalSystem` process executes is a new writable instruction source, which is
[`packaging/the-installer-is-a-sink`](/docs/rules/packaging/running-as-a-service/#the-installer-is-a-sink "The service installer fails closed: a closed serve/run allowlist, no relative path, no argv without --config, no install whose output goes nowhere, no password on a command line") with the check removed.

The encoder is one function with a round-trip property — trailing backslashes, embedded quotes, a
directory path ending in `\` before a closing quote — and a fuzz target. It is
[`core-classes/process-is-argv-only`](/docs/rules/core-classes/processes-and-files/#process-is-argv-only "Core\Process is the one way to run another program, and there is no shell string anywhere in it") inverted: that rule refuses to *build* a command line for a
child, this one has no choice, so the construction is confined to one tested place.

**The binary path is quoted unconditionally**, whether or not it currently contains a space. An
unquoted `ImagePath` under a path with a space is the textbook Windows privilege-escalation finding,
Novis's default install location is such a path, and the finding usually appears after somebody moves
the installation. Arguments given to `sc start <name> arg` reach `ServiceMain` but are not persisted;
nothing may depend on them, and `nvs service run` ignores them.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-classes/processes-and-files/#process-is-argv-only" title="Core\Process is the one way to run another program, and there is no shell string anywhere in it"><code>core-classes/process-is-argv-only</code></a> <a href="/docs/rules/packaging/running-as-a-service/#a-service-is-one-stored-argv" title="nvs service is a namespace, and everything after a mandatory -- is stored verbatim as the argv the service runs"><code>packaging/a-service-is-one-stored-argv</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0093.md">record 0093</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0044.md">record 0044</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-cli/src/service.rs"><code>crates/nvs-cli/src/service.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="a-service-runs-as-a-virtual-account">

## A service's default identity is the per-service virtual account `NT SERVICE\<name>`, and `LocalSystem` is never the default

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#a-service-runs-as-a-virtual-account"><code>packaging/a-service-runs-as-a-virtual-account</code></a>
</div>

The default identity is `NT SERVICE\<name>` — a virtual account the SCM creates and owns, with a
per-service SID, no password to rotate or leak, and no interactive logon. Install grants that SID read
on the config, read/write on the cache and log directories, and nothing further; the account can read
its configuration and write its cache and log, and cannot write its own binary.

`--account` takes a domain identity for a deployment that needs one, with the password prompted rather
than taken from the command line ([`packaging/the-installer-is-a-sink`](/docs/rules/packaging/running-as-a-service/#the-installer-is-a-sink "The service installer fails closed: a closed serve/run allowlist, no relative path, no argv without --config, no install whose output goes nowhere, no password on a command line")). `LocalSystem` is never the
default and must be written out as `--account SYSTEM`.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>A hand-registered service usually runs as <code>LocalSystem</code>; here that must be written out as <code>--account SYSTEM</code>, and the default account has no password to rotate</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/packaging/running-as-a-service/#the-installer-is-a-sink" title="The service installer fails closed: a closed serve/run allowlist, no relative path, no argv without --config, no install whose output goes nowhere, no password on a command line"><code>packaging/the-installer-is-a-sink</code></a> <a href="/docs/rules/packaging/running-as-a-service/#a-service-answers-its-manager" title="A stop drains, a PARAMCHANGE reloads, and lifecycle records go to the event log beside the configured log destination"><code>packaging/a-service-answers-its-manager</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0093.md">record 0093</a></dd></div></dl>

</div>

<div class="nv-rule" id="a-service-answers-its-manager">

## A stop drains, a `PARAMCHANGE` reloads, and lifecycle records go to the event log beside the configured log destination

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#a-service-answers-its-manager"><code>packaging/a-service-answers-its-manager</code></a>
</div>

A hosted server answers its service manager with the operations it already has, rather than a shim
reporting what it can see from outside:

| Control | What the service does |
|---|---|
| stop (`SERVICE_CONTROL_STOP`, `systemctl stop`) | reports `STOP_PENDING` with a checkpoint that advances while requests drain, then `STOPPED` — a machine restart drains in-flight requests instead of killing them |
| `SERVICE_CONTROL_PARAMCHANGE`, `systemctl reload` | performs the configuration reload in-process; the keys it could not apply are written to the event log **by name** ([`config/a-reload-names-what-it-could-not-apply`](/docs/rules/config/reloading-and-control/#a-reload-names-what-it-could-not-apply "A reload reports what it applied, names every changed Boot key it could not apply, and counts the units it invalidated")) |
| `SERVICE_CONTROL_PRESHUTDOWN` | requested at install, because plain `SHUTDOWN` allows roughly five seconds and a drain needs more |

Failure actions are set at install — `--restart on-failure` by default, with a reset period — beside
delayed auto-start (`--start`), dependencies (`--depends-on`, for a database that must come up first)
and a description.

**Output.** With no console handle the process's stderr goes nowhere, so `nvs service run` binds
diagnostics and `Core\Log` to the destination the installer insisted on, and additionally writes a
small, fixed set of lifecycle records — started, stopped, failed to start, reload applied — to the
Windows event log, the first place an administrator looks. The event-log source is registered at
install and removed at uninstall, and `uninstall` leaves nothing behind: no registry key, no source, no
unit file, no granted ACL.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>A shim can only report that the process exited; the service itself reports an advancing stop checkpoint and performs a reload in-process</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/config/reloading-and-control/#a-reload-names-what-it-could-not-apply" title="A reload reports what it applied, names every changed Boot key it could not apply, and counts the units it invalidated"><code>config/a-reload-names-what-it-could-not-apply</code></a> <a href="/docs/rules/concurrency/connections/#a-drain-closes-a-connection-cleanly" title="A shutdown or a reload closes a connection with a defined code after a drain, never with a reset"><code>concurrency/a-drain-closes-a-connection-cleanly</code></a> <a href="/docs/rules/packaging/running-as-a-service/#a-service-runs-as-a-virtual-account" title="A service's default identity is the per-service virtual account NT SERVICE\&lt;name&gt;, and LocalSystem is never the default"><code>packaging/a-service-runs-as-a-virtual-account</code></a> <a href="/docs/rules/http-server/containment/#the-residue-is-one-named-fault-class" title="A memory-safety fault in unsafe code or a miscompile is the one residue: the process dies, the supervisor restarts it, and the answer is frequency, not a boundary"><code>http-server/the-residue-is-one-named-fault-class</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0093.md">record 0093</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0078.md">record 0078</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-cli/src/service.rs"><code>crates/nvs-cli/src/service.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="the-unit-is-printed-and-install-is-the-opt-in">

## On Linux `nvs service unit` prints the systemd unit and touches nothing; writing it is an explicit `--install`

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#the-unit-is-printed-and-install-is-the-opt-in"><code>packaging/the-unit-is-printed-and-install-is-the-opt-in</code></a>
</div>

`nvs service unit <name> -- serve --config …` writes a systemd unit to stdout and touches nothing.
`nvs service install` on Linux is that same generation followed by a write to the system unit
directory and a `daemon-reload` — the explicit request, never the default.

The asymmetry with Windows is deliberate. There, the SCM's own state is the only representation a
service has, so there is no file to hand anyone and installing **is** the feature
([`packaging/the-argv-lives-in-imagepath`](/docs/rules/packaging/running-as-a-service/#the-argv-lives-in-imagepath "On Windows the argv is encoded into the SCM's one ImagePath string, the binary path is always quoted, and nothing else holds it")). On Linux the representation is a text file, the
operator's configuration management already owns the directory it belongs in, and a binary that writes
there and reloads the daemon behind Ansible's back is a worse citizen than one that prints. The
printed unit is also the artifact a change-management review actually wants, which is why `--print`
exists on Windows too, emitting the equivalent `New-Service` invocation for review rather than
execution.

Nothing else is generated: no OpenRC, no SysV, no `rc.d`; `launchd` would take the same printing-only
shape if it is ever added. What the printed unit contains is
[`packaging/the-generated-unit-is-hardened`](/docs/rules/packaging/running-as-a-service/#the-generated-unit-is-hardened "The generated unit carries the hardening block, Type=notify, MemoryMax from [limits], and AmbientCapabilities only for a privileged port").

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>The unit is generated rather than written by hand, and printing rather than installing is the default because configuration management owns the directory</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/packaging/running-as-a-service/#the-generated-unit-is-hardened" title="The generated unit carries the hardening block, Type=notify, MemoryMax from [limits], and AmbientCapabilities only for a privileged port"><code>packaging/the-generated-unit-is-hardened</code></a> <a href="/docs/rules/packaging/running-as-a-service/#the-argv-lives-in-imagepath" title="On Windows the argv is encoded into the SCM's one ImagePath string, the binary path is always quoted, and nothing else holds it"><code>packaging/the-argv-lives-in-imagepath</code></a> <a href="/docs/rules/packaging/running-as-a-service/#a-service-is-one-stored-argv" title="nvs service is a namespace, and everything after a mandatory -- is stored verbatim as the argv the service runs"><code>packaging/a-service-is-one-stored-argv</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0093.md">record 0093</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-cli/src/service.rs"><code>crates/nvs-cli/src/service.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="the-generated-unit-is-hardened">

## The generated unit carries the hardening block, `Type=notify`, `MemoryMax` from `[limits]`, and `AmbientCapabilities` only for a privileged port

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#the-generated-unit-is-hardened"><code>packaging/the-generated-unit-is-hardened</code></a>
</div>

The generated unit carries what a hand-written one usually does not:

```ini
[Service]
Type=notify
ExecStart=/usr/bin/nvs serve --config /etc/nvs/nvs.toml
ExecReload=/usr/bin/nvs ctl reload --socket /run/nvs/control.sock
WatchdogSec=30
User=nvs-web
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
PrivateTmp=true
CapabilityBoundingSet=
AmbientCapabilities=CAP_NET_BIND_SERVICE
RestrictAddressFamilies=AF_INET AF_INET6 AF_UNIX
SystemCallFilter=@system-service
```

`AmbientCapabilities` is emitted **only** when the configured `[server] listen` addresses include a
privileged port, so the ordinary case grants nothing at all. `MemoryMax` is derived from the config's
`[limits]` rather than invented. `Type=notify` means `READY=1` after the listener binds — so
`systemctl start` does not return before the port accepts — plus `RELOADING=1`/`STOPPING=1` at the
transitions and a `WATCHDOG=1` ping for as long as
[`http-server/a-wedged-core-is-detected-by-its-deadline`](/docs/rules/http-server/containment/#a-wedged-core-is-detected-by-its-deadline "A watchdog reads the in-flight deadline each worker already keeps, and reports a core whose oldest deadline is past by a margin")'s detector says a core is still turning.
**That ping is gated on the detector rather than written by an accept loop**, because a beat from a
thread that lives whether or not a core turns proves only that the process exists, which is not what
`WatchdogSec=` is asking. It is withheld when no core is turning at all, and never for one wedged core
of several — that one is shed ([`http-server/a-wedged-core-is-shed-never-killed`](/docs/rules/http-server/containment/#a-wedged-core-is-shed-never-killed "A reported core stops accepting and max_in_flight counts its share as unavailable; a wedged worker is never killed in-process")), and stopping the
whole process over it would end every healthy core's in-flight requests to answer one core's fault.
The `sd_notify` protocol is a datagram to
`$NOTIFY_SOCKET` and needs no `libsystemd`, so this adds no C dependency and
[`packaging/a-c-dependency-answers-two-questions`](/docs/rules/packaging/extensions/#a-c-dependency-answers-two-questions "A C dependency is admitted under ordinary audit only if attacker-controlled data never reaches it; otherwise it needs an exceptional verification record or is confined to wasm") does not arise.

Socket activation — a privileged port with an empty capability set — is deferred rather than refused:
it changes how `nvs serve` acquires its listener, which makes it a server change the unit generator
would simply follow.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p><code>NoNewPrivileges</code>, <code>ProtectSystem=strict</code> and an emptied <code>CapabilityBoundingSet</code> are emitted for you rather than left to whoever writes the unit</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/packaging/running-as-a-service/#the-unit-is-printed-and-install-is-the-opt-in" title="On Linux nvs service unit prints the systemd unit and touches nothing; writing it is an explicit --install"><code>packaging/the-unit-is-printed-and-install-is-the-opt-in</code></a> <a href="/docs/rules/packaging/extensions/#a-c-dependency-answers-two-questions" title="A C dependency is admitted under ordinary audit only if attacker-controlled data never reaches it; otherwise it needs an exceptional verification record or is confined to wasm"><code>packaging/a-c-dependency-answers-two-questions</code></a> <a href="/docs/rules/config/changeability-classes/#three-changeability-classes" title="nvs.toml states defaults, not ceilings, and every directive carries one of three changeability classes"><code>config/three-changeability-classes</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0093.md">record 0093</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0097.md">record 0097</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0051.md">record 0051</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-cli/src/service.rs"><code>crates/nvs-cli/src/service.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="a-bundle-may-not-install-itself">

## A single-file bundle may not install itself as a service

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#a-bundle-may-not-install-itself"><code>packaging/a-bundle-may-not-install-itself</code></a>
</div>

`nvs service install` refuses when the running binary is a single-file bundle, with a diagnostic
(`E0634`) naming the reason.

A bundle is a single trust domain because the person who downloads and runs it is the only principal
involved ([`programs/bundle-trust-domain`](/docs/rules/programs/names-and-files/#bundle-trust-domain "A bundled executable is one trust domain: a program, never a service")). Installing a service creates a **second principal** — a
privileged account executing that payload at every boot, with no operator having read what it
contains. That is the operator-versus-app-author boundary the bundle declined to cross, arrived at from
the other side, and the answer has to be the same one.

The narrower rule — allow it for a non-`serve` payload — is rejected as a conditional a reader must
carry in their head to serve a case nobody has asked for. Reopening this means arguing the trust-domain
point directly.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/programs/names-and-files/#bundle-trust-domain" title="A bundled executable is one trust domain: a program, never a service"><code>programs/bundle-trust-domain</code></a> <a href="/docs/rules/packaging/running-as-a-service/#the-installer-is-a-sink" title="The service installer fails closed: a closed serve/run allowlist, no relative path, no argv without --config, no install whose output goes nowhere, no password on a command line"><code>packaging/the-installer-is-a-sink</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0093.md">record 0093</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0048.md">record 0048</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-cli/src/service.rs"><code>crates/nvs-cli/src/service.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="a-service-is-operator-surface">

## There is no `Core\Service`: only an operator installs a service, from the command line, and nothing about it is on the request path

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#a-service-is-operator-surface"><code>packaging/a-service-is-operator-surface</code></a>
</div>

There is no `Core\Service`, no new grammar, no runtime change and nothing on the request path. A
service manager is not a stdlib candidate under [`core-api/tier-placement`](/docs/rules/core-api/what-belongs-in-core/#tier-placement "A library candidate is placed by six ordered tests, not by PHP's extension list")'s tests at all: a Novis
program cannot install itself as a service, only an operator can, from the command line. That keeps
"an application can never grant itself rights" ([`security/no-runtime-grant`](/docs/rules/security/scopes-and-denial/#no-runtime-grant "Two layers enforce a capability and only the static one grants; nothing anywhere widens")) intact rather than
restating it.

What the feature costs is paid at start and at stop: one control-handler thread with its stack plus a
status structure per served process — kilobytes, O(1), attributable to no request because no request
causes it — and nothing per request. Its dependencies are the two platform crates the
binary already takes for a signal disposition and a console control handler — one Windows-only, one
Unix-only — both behind `#[cfg]`, both pure Rust with no build script, and neither reachable from a
served request. `sd_notify` adds none of its own: it is a datagram of `NAME=value` lines to whatever
`$NOTIFY_SOCKET` names, and it is written by hand.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/security/scopes-and-denial/#no-runtime-grant" title="Two layers enforce a capability and only the static one grants; nothing anywhere widens"><code>security/no-runtime-grant</code></a> <a href="/docs/rules/core-api/what-belongs-in-core/#tier-placement" title="A library candidate is placed by six ordered tests, not by PHP's extension list"><code>core-api/tier-placement</code></a> <a href="/docs/rules/packaging/running-as-a-service/#a-service-is-one-stored-argv" title="nvs service is a namespace, and everything after a mandatory -- is stored verbatim as the argv the service runs"><code>packaging/a-service-is-one-stored-argv</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0093.md">record 0093</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0005.md">record 0005</a></dd></div></dl>

</div>
