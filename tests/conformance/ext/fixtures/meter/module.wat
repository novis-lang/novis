;; The fixture extension `Shop\Meter` as a core module. Its export follows the canonical ABI's
;; names, and the packer componentizes it against `api.wit`.
;;
;; `spin` returns `result<_, error>`, which flattens to more than one value, so it writes the `ok`
;; case to the return area at `$ret` and returns that address.
(module
  (import "wasi:clocks/monotonic-clock@0.2.12" "now" (func $now (result i64)))

  (memory (export "memory") 1)

  ;; The return area: 16 bytes at 16, 8-aligned.
  (global $ret i32 (i32.const 16))

  ;; Nothing the interface passes needs memory from the guest, so nothing is ever allocated.
  (func (export "cabi_realloc")
    (param $old i32) (param $old_size i32) (param $align i32) (param $size i32) (result i32)
    unreachable)

  ;; Reads the monotonic clock until it has moved `$milliseconds`. Each read is a host call, and
  ;; the loop between them never waits, so the whole time is CPU time.
  (func (export "shop:meter/api#spin") (param $milliseconds i64) (result i32)
    (local $until i64)
    (local.set $until
      (i64.add (call $now) (i64.mul (local.get $milliseconds) (i64.const 1000000))))
    (block $done
      (loop $spin
        (br_if $done (i64.ge_u (call $now) (local.get $until)))
        (br $spin)))
    (i32.store8 (global.get $ret) (i32.const 0))
    (global.get $ret)))
