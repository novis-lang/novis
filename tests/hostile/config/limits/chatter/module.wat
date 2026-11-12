;; The attack fixture `Shop\Chatter` as a core module. Its export follows the canonical ABI's
;; names, and the packer componentizes it against `api.wit`.
;;
;; `talk` fills 1 MiB of its memory with `x` and writes it to the request's log, again and again,
;; and never returns. Each line stays on the request until the call returns, so only the request's
;; memory limit ends it.
(module
  (import "nvs:ext/log@1.0.0" "write" (func $write (param i32 i32 i32)))

  ;; 17 pages: the return area in the first, and the line from 65536 on.
  (memory (export "memory") 17)

  ;; Nothing the interface passes needs memory from the guest, so nothing is ever allocated.
  (func (export "cabi_realloc")
    (param $old i32) (param $old_size i32) (param $align i32) (param $size i32) (result i32)
    unreachable)

  ;; Writes the same line at level `debug` without end.
  (func (export "shop:chatter/api#talk") (result i32)
    (memory.fill (i32.const 65536) (i32.const 120) (i32.const 1048576))
    (loop $again
      (call $write (i32.const 0) (i32.const 65536) (i32.const 1048576))
      (br $again))
    unreachable))
