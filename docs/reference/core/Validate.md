---
summary: the format predicates — is this text an email address, a hostname, an IP or MAC address, ASCII, printable — answering `bool` and laundering nothing
keywords: filter_var, FILTER_VALIDATE_EMAIL, FILTER_VALIDATE_IP, FILTER_VALIDATE_DOMAIN, FILTER_VALIDATE_MAC, FILTER_FLAG_IPV4, FILTER_FLAG_IPV6, ctype_print, validation, predicate
---

Every `Core\Validate` member is a predicate over a `string` naming a *format*: `isEmail`,
`isDomain`, `isIp` (with `{version: 4}` or `{version: 6}` to accept one family only), `isMac`,
`isAscii` and `isPrintable`. None launders — a `tainted string` is as tainted after `isEmail`
answers `true` — and none touches the network. There is no `isUrl`, `isInteger` or `isFloat`: those
are `Core\Uri::tryParse($s)?->scheme() != null` and `($s as ?int) != null`.

```nvs
<?nvs
array<string> $emails = ["ada@example.test", "no-at-sign", "ada@localhost"];
foreach ($emails as string $e) {
    echo $e, " ", (Core\Validate::isEmail($e) ? "email" : "not"), "\n";
}
echo (Core\Validate::isDomain("a-b.example.test") ? "domain" : "not"), "\n";
echo (Core\Validate::isIp("192.0.2.1") ? "ip" : "not"), "\n";
echo (Core\Validate::isIp("2001:db8::1", {version: 4}) ? "v4" : "not v4"), "\n";
echo (Core\Validate::isIp("2001:db8::1", {version: 6}) ? "v6" : "not v6"), "\n";
echo (Core\Validate::isMac("00:11:22:33:44:55") ? "mac" : "not"), "\n";
echo (Core\Validate::isAscii("café") ? "ascii" : "not ascii"), "\n";
echo (Core\Validate::isPrintable("café") ? "printable" : "not"), "\n";
echo (Core\Validate::isPrintable("a\tb") ? "printable" : "not"), "\n";
echo (("12" as ?int) != null ? "int" : "not"), "\n";
```
```output
ada@example.test email
no-at-sign not
ada@localhost not
domain
ip
not v4
v6
mac
not ascii
printable
not
int
```
