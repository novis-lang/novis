A request head may have at most 100 header lines, `Host` included. A head with 101 is answered `431 Request Header Fields Too Large`, and the connection is closed. No request is run for it. The bound is `MAX_HEADER_LINES` in `nvs_server::serve`, set on the HTTP parser by name. It is the parser's own default, so a head the server answered before it was named is answered now, and naming it keeps it from moving with the parser's version.

The bound is the parser's and not a size policy. A head is parsed into an array of this many entries, so the number is also what one head costs to parse. `rule:http-server/four-idle-waits-all-finite` leaves a head's size to the proxy, and this rule does not change that: a proxy still owns how many bytes a head may have.

**Declined:**

- **A setting.** No application needs more than a hundred header lines, and one that does is sending state the body should carry.
- **Leaving it to the parser's default unnamed.** The bound would be one no rule and no case states, and a dependency upgrade could move it.
