Turns a link into a full address, the same way a browser does when you click a link on a page. The
address of the page is the base, and you call `resolve` on it. For the base
`https://example.com/blog/post` and the link `../about`, the result is `https://example.com/about`.

A link can be a full address, a path such as `/about` or `photo.jpg`, a query such as `?page=2`, or
a fragment such as `#top`. The result is always a full address with a scheme. The `.` and `..` parts
are removed from its path. The base itself does not change, and its own fragment is not used.

The base must have a scheme, such as `https:`. A base such as `/blog/post` throws a `RuntimeError`.
A link with a character that is not allowed in an address, such as a space, also throws one.

**The examples below** follow the links on a page, remove `.` and `..` from a path, and collect the
links of a page for a crawler.
