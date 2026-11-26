// The mail a `[mail.<name>] capture = true` block kept instead of sending it: the list at `/mail`,
// newest first, and one message at `/mail/<id>`, from `GET /api/mails` and `GET /api/mail`.
//
// The list follows each watermark as the run list does (`ui/list.tsx`). A row shows when the
// message was sent, its block, its subject and its `to` addresses, and `Enter` or a click opens it.
//
// A message's page shows its headers, with Bcc apart because no recipient sees it. The HTML as
// sent draws in an `<iframe sandbox="">`: no script runs, the frame has no origin, and a CSP meta
// element at the top of its document stops anything remote from loading. Each `cid:` link is
// replaced by a `data:` URL of its attachment first, so an inline image shows. The HTML as the
// program wrote it shows as escaped source text only. Below are the text part, the attachments as
// download links, and the CSS report. Escape goes back to the list.
//
// What it spends: the list keeps what `createFeed` keeps, a few hundred bytes a row. A message's
// page keeps the one message, its attachments' base64 and the frame's document, with each inline
// image's base64 a second time, for as long as the page is open.

import { createResource, createSignal, For, Match, onCleanup, onMount, Show, Switch } from "solid-js";
import { ApiError, mail, mails, type MailAttachment, type MailMessage, type MailRow } from "../api.ts";
import { ignored } from "../keys.ts";
import { mailLink, navigate } from "../router.ts";
import { Chip } from "../ui/badges.tsx";
import { Empty } from "../ui/empty.tsx";
import { createFeed, List, type RowKey } from "../ui/list.tsx";
import { age, createNow, size } from "./requests.tsx";

/** The size above which Gmail cuts a message and links to the rest: 102 KB. */
export const GMAIL_CLIP = 102 * 1024;

/** The first element of every sent HTML document the frame shows: nothing loads from the network,
 *  an image loads only from a `data:` URL, and only inline CSS applies. */
export const FRAME_CSP = `<meta http-equiv="Content-Security-Policy" content="default-src 'none'; img-src data:; style-src 'unsafe-inline'">`;

/** A MIME type safe to write into a `data:` URL inside an HTML attribute. */
function safeType(type: string): string {
  const clean = type.replace(/[^\w.+/=;-]/g, "");
  return clean || "application/octet-stream";
}

/** The `data:` URL of an attachment's bytes. */
export function dataUrl(file: MailAttachment): string {
  return `data:${safeType(file.type)};base64,${file.base64}`;
}

/** The document the frame shows: `html` with each `cid:<content id>` replaced by the `data:` URL of
 *  its attachment, and the CSP meta element first. A doctype stays first, so the page keeps its mode. */
export function frameDocument(html: string, attachments: readonly MailAttachment[] = []): string {
  // The longest id first, so an id that starts another one never replaces part of it.
  const inline = attachments.filter((file) => file.content_id).sort((a, b) => b.content_id!.length - a.content_id!.length);
  let out = html;
  for (const file of inline) out = out.split(`cid:${file.content_id}`).join(dataUrl(file));
  const doctype = /^\s*<!doctype[^>]*>/i.exec(out);
  if (doctype) return doctype[0] + FRAME_CSP + out.slice(doctype[0].length);
  return FRAME_CSP + out;
}

/** The size of `text` in bytes, as UTF-8. */
function bytes(text: string): number {
  return new TextEncoder().encode(text).length;
}

/** The sender the way a mail program shows it: `"Name" <address>`, or the address alone. */
export function fromLine(message: Pick<MailMessage, "from" | "from_name">): string {
  if (!message.from_name) return message.from;
  return `"${message.from_name.replace(/(["\\])/g, "\\$1")}" <${message.from}>`;
}

/** When `us` (microseconds since the epoch) was, as a `<time>` that shows its age. */
function Sent(props: { us: number; now: number }) {
  const date = () => new Date(props.us / 1000);
  return (
    <time datetime={date().toISOString()} title={date().toLocaleString()}>
      {age(props.us, props.now)} ago
    </time>
  );
}

function MailColumns() {
  return (
    <>
      <span>Sent</span>
      <span>Block</span>
      <span>Subject</span>
      <span>To</span>
    </>
  );
}

function MailCells(props: { row: MailRow; now: number }) {
  const row = () => props.row;
  return (
    <>
      <span class="age">
        <Sent us={row().at_us} now={props.now} />
      </span>
      <span class="mail-block">{row().endpoint}</span>
      <span class="mail-subject" title={row().subject}>
        {row().subject}
      </span>
      <span class="mail-to" title={row().recipients}>
        {row().recipients}
      </span>
    </>
  );
}

export function MailList() {
  const feed = createFeed<MailRow, number>({
    load: async (at, signal) => {
      const page = await mails(at, signal);
      return { rows: page.mails, older: page.older, watermark: page.watermark };
    },
    key: (row) => row.id,
    cursor: (row) => row.id,
  });
  const [selected, setSelected] = createSignal<RowKey | null>(null);
  const now = createNow();

  return (
    <section class="requests mail-list" aria-label="Mail">
      <div class="requests-body">
        <List
          rows={feed.rows()}
          key={(row) => row.id}
          idPrefix="mail"
          label="Kept mail, newest first"
          selected={selected()}
          onSelect={setSelected}
          onOpen={(row) => navigate(mailLink(row))}
          openOnClick
          grid="mail-grid"
          cols={<MailColumns />}
          feed={feed}
          noun={["message", "messages"]}
          empty={
            <Empty icon="mail" title="No mail kept yet">
              <p>
                A mail block with <code>capture = true</code> keeps each message here and sends nothing. Add it to a block in{" "}
                <code>nvs.toml</code>:
              </p>
              <pre class="empty-code">{"[mail.outbox]\ncapture = true"}</pre>
            </Empty>
          }
        >
          {(row) => <MailCells row={row} now={now()} />}
        </List>
      </div>
    </section>
  );
}

export function MailView(props: { id: number }) {
  const [detail] = createResource(() => props.id, (id) => mail(id));
  const missing = () => detail.error instanceof ApiError && detail.error.status === 404;
  const now = createNow();

  const onKey = (e: KeyboardEvent) => {
    if (ignored(e) || e.key !== "Escape") return;
    navigate("/mail");
    e.preventDefault();
  };
  onMount(() => document.addEventListener("keydown", onKey));
  onCleanup(() => document.removeEventListener("keydown", onKey));

  return (
    <section class="request mail-one" aria-label="Message">
      <Switch>
        <Match when={missing()}>
          <p class="note">No kept message has this id.</p>
        </Match>
        <Match when={detail.error}>
          <p class="note bad" role="alert">
            The message could not load: {String((detail.error as Error | undefined)?.message ?? detail.error)}
          </p>
        </Match>
        <Match when={detail.loading}>
          <p class="note">Loading.</p>
        </Match>
        <Match when={detail()}>
          {(d) => (
            <>
              <header class="req-head">
                <div class="req-title">
                  <h2 class="title">
                    <span class="req-path">{d().message.subject}</span>
                  </h2>
                </div>
                <p class="req-crumb">
                  <Sent us={d().at_us} now={now()} />
                  {" · "}Block <code>{d().message.endpoint}</code>
                  {" · "}Trace <code class="req-id">{d().trace}</code>
                </p>
              </header>
              <div class="req-panel">
                <Message message={d().message} />
              </div>
            </>
          )}
        </Match>
      </Switch>
    </section>
  );
}

/** One header line of the table, shown only when it has a value. */
function HeaderRow(props: { name: string; value: string | undefined }) {
  return (
    <Show when={props.value}>
      <tr>
        <th scope="row">{props.name}</th>
        <td class="rt-code">{props.value}</td>
      </tr>
    </Show>
  );
}

/** Everything below the page's title: the headers, the HTML, the text, the attachments and the
 *  CSS report. */
function Message(props: { message: MailMessage }) {
  const m = () => props.message;
  const list = (items: string[] | undefined) => (items && items.length > 0 ? items.join(", ") : undefined);
  return (
    <div class="rt">
      <Show when={m().elided}>
        <p class="note" role="note">
          This message was too large to keep whole. Its HTML, its text and the contents of its attachments were removed.
        </p>
      </Show>
      <h3 class="rt-head">Headers</h3>
      <table class="rt-table mail-headers" aria-label="Headers">
        <tbody>
          <HeaderRow name="From" value={fromLine(m())} />
          <HeaderRow name="To" value={list(m().to)} />
          <HeaderRow name="Cc" value={list(m().cc)} />
          <HeaderRow name="Reply-To" value={m().reply_to} />
          <HeaderRow name="Subject" value={m().subject} />
          <HeaderRow name="Message-ID" value={`<${m().message_id}>`} />
          <HeaderRow name="In-Reply-To" value={m().in_reply_to ? `<${m().in_reply_to}>` : undefined} />
          <HeaderRow name="References" value={m().references?.map((id) => `<${id}>`).join(" ") || undefined} />
          <HeaderRow name="List-Unsubscribe" value={m().unsubscribe ? `<${m().unsubscribe}>` : undefined} />
          <HeaderRow name="Auto-Submitted" value={m().auto_submitted ? "auto-generated" : undefined} />
          <HeaderRow name="Bounce tag" value={m().bounce_tag} />
        </tbody>
      </table>
      <Show when={list(m().bcc)}>
        {(bcc) => (
          <section class="mail-bcc" aria-label="Bcc">
            <h3 class="rt-head">Bcc</h3>
            <p class="note">These addresses got the message. The other recipients do not see them.</p>
            <p class="rt-code">{bcc()}</p>
          </section>
        )}
      </Show>
      <Show when={m().html !== undefined || m().html_written !== undefined}>
        <Html message={m()} />
      </Show>
      <h3 class="rt-head">Text</h3>
      <Show when={m().text} fallback={<p class="note">The message has no text part.</p>}>
        <pre class="body-text mail-text">{m().text}</pre>
      </Show>
      <Show when={(m().attachments ?? []).length > 0}>
        <Attachments files={m().attachments!} />
      </Show>
      <Show when={(m().kept_css ?? []).length > 0 || (m().unused_css ?? []).length > 0}>
        <CssReport kept={m().kept_css ?? []} unused={m().unused_css ?? []} />
      </Show>
    </div>
  );
}

/** The HTML part: as sent in a sandboxed frame, or as the program wrote it as source text. */
function Html(props: { message: MailMessage }) {
  const m = () => props.message;
  const [written, setWritten] = createSignal(m().html === undefined);
  const sentSize = () => bytes(m().html ?? "");
  return (
    <section class="mail-html" aria-label="HTML">
      <h3 class="rt-head">HTML</h3>
      <div class="filter-chips" role="group" aria-label="Which HTML to show">
        <Show when={m().html !== undefined}>
          <Chip set={!written()} onClick={() => setWritten(false)}>
            As sent
          </Chip>
        </Show>
        <Show when={m().html_written !== undefined}>
          <Chip set={written()} onClick={() => setWritten(true)}>
            As written
          </Chip>
        </Show>
      </div>
      <Show when={sentSize() > GMAIL_CLIP}>
        <p class="note">
          The HTML as sent is {size(sentSize())}. Gmail cuts a message larger than 102 KB and shows a link to the rest.
        </p>
      </Show>
      <Show
        when={written()}
        fallback={
          <>
            <p class="note">Scripts do not run in this preview. Nothing loads from the network.</p>
            <iframe class="mail-frame" sandbox="" title="The HTML as sent" srcdoc={frameDocument(m().html ?? "", m().attachments)} />
          </>
        }
      >
        <p class="note">This is the HTML your program wrote. In the HTML as sent, its CSS is copied into the elements it styles.</p>
        <pre class="body-text mail-source">{m().html_written ?? ""}</pre>
      </Show>
    </section>
  );
}

/** Each attachment as a download link, with its type and size. */
function Attachments(props: { files: MailAttachment[] }) {
  return (
    <>
      <h3 class="rt-head">Attachments</h3>
      <table class="rt-table mail-files" aria-label="Attachments">
        <thead>
          <tr>
            <th scope="col">Name</th>
            <th scope="col">Type</th>
            <th scope="col" class="rt-num">
              Size
            </th>
          </tr>
        </thead>
        <tbody>
          <For each={props.files}>
            {(file) => (
              <tr>
                <td>
                  <a download={file.name} href={dataUrl(file)}>
                    {file.name}
                  </a>
                  <Show when={file.content_id}>
                    {" "}
                    <span class="chip mail-inline" title="The HTML shows this image inside the message.">
                      inline
                    </span>
                  </Show>
                </td>
                <td class="rt-code">{file.type}</td>
                <td class="rt-num">{size(file.size)}</td>
              </tr>
            )}
          </For>
        </tbody>
      </table>
    </>
  );
}

/** The CSS rules that stayed in a `<style>` element, and the ones of those that matched nothing. */
function CssReport(props: { kept: string[]; unused: string[] }) {
  return (
    <section class="mail-css" aria-label="CSS">
      <h3 class="rt-head">CSS</h3>
      <Show when={props.kept.length > 0}>
        <h4>Rules kept in a style element</h4>
        <p class="note">
          These rules stay in a <code>{"<style>"}</code> element because they cannot be copied into a <code>style</code> attribute. A media
          query and a rule with <code>:hover</code> are examples. Some mail programs ignore a <code>{"<style>"}</code> element.
        </p>
        <ul class="mail-rules mail-kept">
          <For each={props.kept}>{(rule) => <li class="rt-code">{rule}</li>}</For>
        </ul>
      </Show>
      <Show when={props.unused.length > 0}>
        <h4>Rules that match no element</h4>
        <p class="note">These rules stay in the style element, but no element in this message matches them. You can delete them.</p>
        <ul class="mail-rules mail-unused">
          <For each={props.unused}>{(rule) => <li class="rt-code">{rule}</li>}</For>
        </ul>
      </Show>
    </section>
  );
}
