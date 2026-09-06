A secret file is a configuration input like any other, so it passes
`rule:config/ownership-is-the-trust-boundary` before it is read: **group- or world-writable is a
refusal**, since another account able to rewrite the file chooses the credential the server connects
with.

Group- or world-*readable* is a warning naming the mode, not a refusal. Docker Compose mounts secrets
`0444` and Kubernetes secret volumes default to `0644`; inside a container that is the norm, on a
shared host it is not, and nothing readable from the runtime says which one it is in. Refusing would
wall off every containerised deployment; saying nothing would hide a real mistake on a shared host.
**Integrity is enforced; confidentiality is advised.** The advisory is printed in full by `nvs config
check` and counted on its summary line, and it never changes the verdict.
