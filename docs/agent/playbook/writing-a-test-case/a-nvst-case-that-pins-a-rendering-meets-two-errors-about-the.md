- **A `.nvst` case that pins a rendering meets two errors about the case rather than the member.**
  `Core\Debug::render` answers `Core\Cli\Text`, not `string`, so `string $r =
  Core\Debug::render($v);` is `E0401`; and a rendered property name begins with `$`, so `"{\n $token
  => …"` is `E0301: $token is not declared`. Escape the property as `\$token`;
  `test-assert-matches-inline-never-holds-a-secret-property.nvst` carries the spelling.
  [until: gone crates/nvs-stdlib/src/debug.rs:Core\Debug::render]
