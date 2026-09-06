`nvs service unit <name> -- serve --config …` writes a systemd unit to stdout and touches nothing.
`nvs service install` on Linux is that same generation followed by a write to the system unit
directory and a `daemon-reload` — the explicit request, never the default.

The asymmetry with Windows is deliberate. There, the SCM's own state is the only representation a
service has, so there is no file to hand anyone and installing **is** the feature
(`rule:packaging/the-argv-lives-in-imagepath`). On Linux the representation is a text file, the
operator's configuration management already owns the directory it belongs in, and a binary that writes
there and reloads the daemon behind Ansible's back is a worse citizen than one that prints. The
printed unit is also the artifact a change-management review actually wants, which is why `--print`
exists on Windows too, emitting the equivalent `New-Service` invocation for review rather than
execution.

Nothing else is generated: no OpenRC, no SysV, no `rc.d`; `launchd` would take the same printing-only
shape if it is ever added. What the printed unit contains is
`rule:packaging/the-generated-unit-is-hardened`.
