Returns the limit that a rate limit decision was made against.

When you call `Core\RateLimit::shed` or `Core\RateLimit::consume`, you pass the number of units a
key may use in a period. The `Core\RateLimit\Decision` you get back contains that number, and
`limit` returns it unchanged. It is the same for a call that was allowed and for one that was not.

This is useful when the code that writes the response does not know which limit was used. A
function can write the `RateLimit-Limit` header from the decision alone, whatever plan the user has.
