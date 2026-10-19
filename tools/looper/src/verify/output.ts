// A command's output kept within a bound: its first and its last `OUTPUT_HALF` bytes, with the middle
// dropped and counted. Verify holds a cargo command's output this way, and acceptance each stream of a
// check's command and a file check's text.
//
// # The bounds it keeps
//
// - A `Bounded` holds at most `2 * OUTPUT_HALF` bytes, however much it is given.

/** The bytes of a stream kept at its head, and again at its tail. */
export const OUTPUT_HALF = 4 * 1024 * 1024;

/** One stream's head and tail. */
export class Bounded {
  private readonly head: Buffer[] = [];
  private headBytes = 0;
  private tail: Buffer[] = [];
  private tailBytes = 0;
  private dropped = 0;

  constructor(private readonly half: number = OUTPUT_HALF) {}

  /** Keeps what of `chunk` falls in the head or the tail. */
  push(chunk: Buffer): void {
    if (this.headBytes < this.half) {
      const take = chunk.subarray(0, this.half - this.headBytes);
      this.head.push(Buffer.from(take));
      this.headBytes += take.length;
      chunk = chunk.subarray(take.length);
    }
    if (chunk.length === 0) return;
    this.tail.push(Buffer.from(chunk));
    this.tailBytes += chunk.length;
    while (this.tailBytes > this.half) {
      const first = this.tail[0] as Buffer;
      const over = this.tailBytes - this.half;
      if (first.length <= over) {
        this.tail.shift();
        this.tailBytes -= first.length;
        this.dropped += first.length;
      } else {
        this.tail[0] = first.subarray(over);
        this.tailBytes -= over;
        this.dropped += over;
      }
    }
  }

  /** The head, a line that counts the bytes dropped when any were, and the tail. */
  text(): string {
    const gap = this.dropped > 0 ? `\n[... ${this.dropped} bytes dropped ...]\n` : "";
    return `${Buffer.concat(this.head).toString("utf8")}${gap}${Buffer.concat(this.tail).toString("utf8")}`;
  }
}
