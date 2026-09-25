- **A `Core` member's `signature` in `nvs meta --json` carries its type parameters, so the leading
  token of a rendered line is not the member's name.** `Core\Json::decodeAs<T>(string $json): T` is
  the spec's own spelling, and a symbol read as "everything before the first `(`" keeps the `<T>` and
  then resolves to nothing — three of `nvs agent`'s cases went red on the ten `…As` members at once.
  Split on `['(', '<', ' ']` when reading a name back out of a rendered signature, and check
  `Core\Arr::shapeAs` and `Core\Request::queryAs` before believing a walk over the registry is
  complete. [until: gone crates/nvs-stdlib/src/request.rs:Core\Request::queryAs]
