`url()` returns the URL of one request that your program sent in a test. You get the request from
`Core\Test::sentHttp()`, after `Core\Test::answerHttp` has set up a fake web service.

The result is the whole URL, exactly as the program wrote it. It includes the path and the query
string. It is not the pattern you gave to `Core\Test::answerHttp`. When one pattern ending in `*`
answers many URLs, `url()` still tells you which URL the program used. This lets a test check that
the program built a path or a query string correctly.

**The examples below** show how to read one URL, the URLs behind one pattern, and a test that
checks a search client encodes its search text.
