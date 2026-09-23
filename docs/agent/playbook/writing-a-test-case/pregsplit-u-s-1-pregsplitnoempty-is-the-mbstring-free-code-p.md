- **`preg_split("//u", $s, -1, PREG_SPLIT_NO_EMPTY)` is the mbstring-free code point splitter, and
  works on the Windows `php`.** PCRE carries its own UTF-8 support, so a `Core\Str` slice that needs
  PHP to count code points has a real oracle rather than a frozen `--EXPECT--`. What it cannot give
  is a code point's *number* (`mb_ord`) or a grapheme (`grapheme_strlen`: no `intl` either), so an
  oracle needing those still decodes UTF-8 by hand in the `--ORACLE--` block or freezes the rows and
  cites the UCD table. [until: reviewed 2026-09-06]
