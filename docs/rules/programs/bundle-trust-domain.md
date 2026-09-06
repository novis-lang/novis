`nvs build --compile` ships a runnable *program*, not a deployable *service*. The person who downloads
and runs the resulting executable is the only principal involved, exactly as for any other native
binary, so the root-owned-configuration-versus-app-capability separation is not engaged at all: there
is no operator-versus-author boundary inside a bundle to protect, and this feature introduces no
capability model of its own.

**`nvs serve` is not bundled.** Packaging a web-serving deployment this way would put the app's own
build step in control of what ships as the equivalent of a root-owned configuration, which is precisely
the property that an application can never grant itself rights. That is a different feature needing its
own argument, not a generalisation of this one.

The same boundary holds from the other side: **a bundled executable may not install itself as a
service.** A privileged account executing that payload at every boot is exactly the second principal
the single-trust-domain argument depends on there not being, so the installer refuses a host that is
itself a bundle.
