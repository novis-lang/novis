---
summary: RFC 3986 URI references read, rebuilt, resolved and compared, with the two percent-encoders and PHP's query-string convention
keywords: parse_url, filter_var, FILTER_VALIDATE_URL, rawurlencode, rawurldecode, urlencode, urldecode, parse_str, http_build_query, URL, URI, query string, percent-encoding
---

`Core\Uri::parse` reads a URI *reference* and answers an instance whose readers report each
component exactly as written — still percent-encoded, in its own case. Every reader but `path` is
`?string` (`port` is `?int`) and answers `null` where the component was not written; `tryParse` is
the same read with `null` in place of `RuntimeError`, and is the spelling of "is this text a URI".
`with` and `resolve` answer a fresh `Uri`, and `compareTo` is the normalized content comparison —
`==` on two `Uri` objects is identity. `parseQuery` and `buildQuery` read and write the bracket
convention, and `queryParameter`/`withQueryParameter` are the same convention over **one** parameter
by name — a name `with` cannot take as a bag key, because a bag's keys are declared and a parameter's
name is chosen at run time. The name is top-level, so an array `$value` is what writes `a[b]`; a
`null` `$value` removes the pair, and removing the last one leaves no query at all rather than a bare
`?`. `encodeComponent`/`encodeFormValue` are `rawurlencode`'s and `urlencode`'s two escapes.
The two **decoders answer `bytes`**, because percent-decoding is defined over octets and a client may
send any of them: `Core\Uri::decodeComponent("%FF")` has an answer, and text is one `as string` away —
which throws for octets no `string` can hold, exactly where a `string`-returning decoder would have.
`parseQuery` answers those same octets for a value; a name is the array key, so it is a `string`.

```nvs
<?nvs
var $u = Core\Uri::parse("https://example.com:8443/a/b%20c?x=1&y=2#top");
echo $u->scheme() ?? "-", " ", $u->host() ?? "-", " ", $u->port() ?? 0, "\n";
echo $u->path(), " ", $u->query() ?? "-", " ", $u->fragment() ?? "-", "\n";
echo $u->with({path: "/c", fragment: "end"})->toString(), "\n";
echo $u->resolve("../d?z=3")->toString(), "\n";

var $rel = Core\Uri::parse("/only/a/path");
echo $rel->scheme() == null ? "relative" : "absolute", "\n";
echo Core\Uri::tryParse("http://example.com/a b") == null ? "not a uri" : "a uri", "\n";

var $same = Core\Uri::parse("HTTPS://EXAMPLE.com:8443/a/b%20c?x=1&y=2#top");
echo $u->compareTo($same) == 0 ? "same uri" : "different", "\n";

var $q = Core\Uri::parseQuery("a=1&b[]=2&b[]=3");
echo Core\Str::join(Core\Arr::keys($q), ","), "\n";
array<mixed> $params = ["name" => "a b", "page" => 2];
echo Core\Uri::buildQuery($params), "\n";
echo $u->queryParameter("x") as bytes as string, "\n";
echo $u->withQueryParameter("y", null)->withQueryParameter("z", "3")->toString(), "\n";
echo Core\Uri::encodeComponent("a b&c"), " ", Core\Uri::encodeFormValue("a b&c"), "\n";
```
```output
https example.com 8443
/a/b%20c x=1&y=2 top
https://example.com:8443/c?x=1&y=2#end
https://example.com:8443/d?z=3
relative
not a uri
same uri
a,b
name=a+b&page=2
1
https://example.com:8443/a/b%20c?x=1&z=3#top
a%20b%26c a+b%26c
```
