// The local origin `examples/http.nvs` talks to: `http://127.0.0.1:8099`, one route, one answer, each held
// `DELAY_MS` before it goes out. The example names that address rather than discovering it, so the sweep
// holds this listener up around its checks, in its own process, and closes it when they are done. Run by
// hand with `bun nv origin` to work on the example; it serves until the process is stopped.
//
// **The hold is what makes the example's last line deterministic.** The example asks the same URL for an
// answer inside 1ms, and the point it makes is that the budget, not the server, ends the call. An origin
// on loopback that answered at once would turn that line into a race between a 1ms deadline and a
// sub-millisecond round trip. The hold is far under the default deadline the first request gets and far
// over the one the last request asks for, so both lines are decided by arithmetic.
//
// The reply is written by hand: `crates/nvs-stdlib/src/http/transport.rs` parses it, so what this serves is
// a test input to a parser, framed by `Content-Length` as the transport's own unit tests frame theirs.
// `/ok` answers 200 `ok` and every other path 404, because an origin that says yes to everything could not
// show that pinning, not the route, refused a request.
//
// When something already answers on the port — an origin started by hand and left up — it is left alone
// and `holdOrigin`'s handle closes nothing.

export const HOST = "127.0.0.1";
export const PORT = 8099;
export const DELAY_MS = 25;
/** The programs that talk to this origin: a check that runs one needs it up. */
export const ORIGIN_PROGRAMS: readonly string[] = ["examples/http.nvs"];
/** The request head is read to its blank line, and no further than this. */
const HEAD_LIMIT = 8192;

export interface Origin {
  /** The line that says what is serving, for the caller to print. */
  line: string;
  close(): void;
}

function reply(status: number, phrase: string, body: string): string {
  return `HTTP/1.1 ${status} ${phrase}\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: ${Buffer.byteLength(body)}\r\nConnection: close\r\n\r\n${body}`;
}

/** Whether a listener already answers at `host:port`. */
async function answering(host: string, port: number): Promise<boolean> {
  try {
    const probe = await Promise.race([
      Bun.connect({ hostname: host, port, socket: { data() {}, error() {} } }),
      Bun.sleep(1000).then(() => null),
    ]);
    if (probe === null) return false;
    probe.end();
    return true;
  } catch {
    return false;
  }
}

/** The origin, up until `close`. A port it cannot bind is reported in `line` and fails no one here. */
export async function holdOrigin(host = HOST, port = PORT, delayMs = DELAY_MS): Promise<Origin> {
  if (await answering(host, port)) return { line: `origin: http://${host}:${port} already has a listener -- leaving it alone`, close() {} };
  try {
    const server = Bun.listen<{ head: string; answered: boolean }>({
      hostname: host,
      port,
      socket: {
        open(s) {
          s.data = { head: "", answered: false };
        },
        data(s, chunk) {
          if (s.data.answered) return;
          s.data.head += Buffer.from(chunk).toString("latin1");
          if (!s.data.head.includes("\r\n\r\n") && s.data.head.length < HEAD_LIMIT) return;
          s.data.answered = true;
          const path = s.data.head.split("\r\n", 1)[0]!.split(" ")[1] ?? "";
          setTimeout(() => {
            // The caller may have hung up inside the hold, which is how the example's last request ends.
            try {
              s.write(path === "/ok" ? reply(200, "OK", "ok") : reply(404, "Not Found", "not found"));
              s.end();
            } catch {}
          }, delayMs);
        },
        error() {},
      },
    });
    return { line: `origin: serving http://${host}:${port}/ok (${delayMs}ms per answer)`, close: () => server.stop(true) };
  } catch (e) {
    return { line: `origin: cannot bind ${host}:${port} -- ${e instanceof Error ? e.message : String(e)}`, close() {} };
  }
}

/** Runs `fn` with the origin up, and closes it when `fn` settles, whether it returned or threw. */
export async function withOrigin<T>(fn: (origin: Origin) => Promise<T>, host = HOST, port = PORT, delayMs = DELAY_MS): Promise<T> {
  const origin = await holdOrigin(host, port, delayMs);
  try {
    return await fn(origin);
  } finally {
    origin.close();
  }
}
