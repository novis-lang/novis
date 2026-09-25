- **A sweep over `Core\Json::decodeAs<T>` needs one helper per `T`, not one helper.** The class is
  written at the call site (`rule:core-api/shape-rules` R4; `WRITTEN_CLASS_MEMBERS`), so no
  parameter can carry it. Write one `try { … return true; } catch (Throwable $bad) { return false;
  }` per type over the same document builder and sweep the *documents*, as
  `tests/conformance/core/json-decode-as-admits-exactly-its-declared-type.nvst` does.
  [until: gone tests/conformance/core/json-decode-as-admits-exactly-its-declared-type.nvst:rule:core-api/shape-rules]
