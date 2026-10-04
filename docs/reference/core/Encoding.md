---
summary: the seam between `string` and `bytes` — charset conversion, and base64, base64url, base32 and hex spellings of a buffer
keywords: iconv, mb_convert_encoding, mb_check_encoding, utf8_encode, utf8_decode, base64_encode, base64_decode, bin2hex, hex2bin, unpack("H*"), base32, charset, UTF-8, Latin1, ISO-8859-1, Windows-1252, UTF-16, Shift_JIS
---

`Core\Encoding` is where a `string` becomes `bytes` and back: `encodeText` and `decodeText` convert
text through a `Core\Charset` case, and the base64, base64url, base32 and hex pairs spell a buffer
as text and read it back. Every decoder is strict — a character outside the alphabet, padding that
is missing or non-canonical, a sequence the charset cannot read — and throws rather than answering
`false` or substituting `?` or U+FFFD; `isValidText` is `decodeText`'s question with a `bool`
answer. A `bytes` value cannot be written directly in the code. Use `fromHex` or `encodeText` to
write a buffer in source.

```nvs
<?nvs
bytes $raw = Core\Encoding::encodeText("héllo", Core\Charset::Utf8);
echo Core\Bytes::length($raw), " ", Core\Encoding::toHex($raw), "\n";
echo Core\Encoding::toBase64($raw), " ", Core\Encoding::toBase64Url($raw), " ", Core\Encoding::toBase32($raw), "\n";
echo Core\Encoding::decodeText(Core\Encoding::fromBase64("aMOpbGxv"), Core\Charset::Utf8), "\n";
bytes $latin = Core\Encoding::encodeText("héllo", Core\Charset::Latin1);
echo Core\Bytes::length($latin), " ", Core\Encoding::toHex($latin), "\n";
echo Core\Encoding::isValidText($latin, Core\Charset::Utf8) ? "utf-8" : "not utf-8", "\n";
bytes $png = Core\Encoding::fromHex("89504e47");
echo Core\Bytes::at($png, 1), "\n";
try {
    bytes $never = Core\Encoding::fromBase64("not base64!");
    echo "decoded\n";
} catch (Throwable $e) {
    echo "refused\n";
}
```
```output
6 68c3a96c6c6f
aMOpbGxv aMOpbGxv NDB2S3DMN4
héllo
5 68e96c6c6f
not utf-8
80
refused
```
