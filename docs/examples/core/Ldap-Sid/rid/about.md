Returns the last number of the SID, the RID (relative identifier). In a domain, the RID tells the
accounts apart. Some RIDs are the same in every domain: `500` is the built-in `Administrator`,
`512` is `Domain Admins` and `513` is `Domain Users`.

Use it to find a well-known account or group, whatever its domain is.

**Good to know:** the RID is a number from `0` to `4294967295`.
