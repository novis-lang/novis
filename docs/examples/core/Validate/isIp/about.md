Checks whether a text is an IP address. An IP address is the number of a computer on a network.
The method returns `true` or `false`.

There are two versions of IP addresses. An IPv4 address is four numbers from 0 to 255 joined by
dots, such as `192.0.2.1`. An IPv6 address is up to eight groups of hex digits joined by `:`, such
as `2001:db8::1`. A `::` replaces a run of groups that are all zero.

Without options, both versions are accepted. The option `version` set to `4` accepts only IPv4, and
set to `6` accepts only IPv6.

A number with a leading zero, such as `192.0.2.01`, gives `false`. So does a space around the
address, or a zone at the end, such as `fe80::1%eth0`. The method does not check whether the
address exists.

**The examples below** check a few texts, accept only one version, and check an allow list that
somebody typed in.
