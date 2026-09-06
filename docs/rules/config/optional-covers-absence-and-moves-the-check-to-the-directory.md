**`optional = true` covers absence and nothing else.** A file that exists but cannot be read, that fails
the ownership check, or that fails to parse is a hard refusal — otherwise a stray `chmod` silently drops
half a configuration and the server comes up looking healthy.

Because an absent file offers nothing to check, **the check falls on the directory that would contain
it**:

```console
E0607: [[include]] /etc/nvs/local.toml is optional, but /etc/nvs is group-writable
       (mode 0775, group `deploy`)
       an absent optional include is a standing slot that anyone able to write that
       directory may later fill with root-owned configuration
       fix: chmod 0755 /etc/nvs
```

That refusal is the whole security argument for the feature. An optional include is a promise that a
file which does not exist yet will be trusted when it appears; the only place to make that promise
safely is the directory, and it is the one place a check can still run. When that directory does not
exist either, the check walks up to the nearest ancestor that does: the promise is only as strong as
the shallowest directory an attacker would have to write in order to keep it. A `dir` include's
directory is checked the same way.
