Every owner is notified of each publish, new token, owner change, unlist and retraction through the
channels that owner set up — **Web Push** and **webhooks** — and an account with no verified channel
cannot publish and cannot be added as an owner.

**The channels an owner sets up:**

- **Web Push**, one click in the browser. Each message is sent through `Core\WebPush`
  (`rule:core-classes/web-push-sends-an-encrypted-message`), encrypted to the device and signed with the
  registry's VAPID key. An owner may register any number of devices. A device is added only while signed
  in with a passkey, and adding or removing one notifies every other device and webhook of that account.
  A subscription the push service reports gone is dropped, and the account page lists each device with
  the date it last answered. The account page subscribes the browser with the key from `/api/push/key`
  and adds, tests, verifies and removes each device and webhook
  (`rule:packaging/the-registry-is-a-novis-program`).
- **Webhooks**: a signed POST to a URL the owner gives, such as ntfy, Matrix, Discord or the owner's own
  server. It is sent through the outbound address table, so an internal host is refused, and it is
  retried. Each message also carries a one-time "I am here" link
  (`rule:packaging/an-account-stays-active-by-proof-not-by-token`).

**Always present, whatever the owner set up:** a signed feed per vendor, the inbox on the website, and
`nvs pkg publish` and `nvs pkg whoami` printing the pending releases the caller did not confirm. A
public feed lists every pending release (`rule:packaging/a-release-waits-seven-days-unless-an-owner-confirms`).
`nvs pkg publish` warns when no device or webhook of an owner of the vendor took the message.

**One message per event, saved with the change.** Each change writes a `notices` row in the
transaction that writes the change and its log entry, so an event is never shown without its message.
The route sends every unsent row once it commits, and claims each row before it sends it, so two
requests never send one message twice. The every-minute job sends what a failed request left. A message
goes to every owner of the vendor and to the account the event is about, such as a removed owner or the
maker of a token.

**The feeds are C2SP signed notes under the log's key**, the format of the log's checkpoints:
`GET /feeds/vendors/<vendor>` has the newest 100 events and `GET /feeds/pending` every pending release.
Each line after the name and the count is one JSON object. The log's public key checks a feed, with the
client's note reader or any C2SP library. So a copy of a feed can be checked, wherever it came from. The
inbox, `GET /api/inbox`, is the same rows for a signed-in account.

**A verified channel is mandatory.** A push device is verified when its test push arrives: the
registry's service worker sends back the one-time code inside the encrypted payload, which only a
subscription a passkey session registered can decrypt. A reminder's "Yes, I am here" stays a click on
the message, because that answer proves a person is there. A webhook is verified by a one-time code the
registry sends to it, which the owner returns on the website or through the link in the message. **A
channel that stops working stops counting**: the push service reports it gone, or a webhook still fails
after its retries. An account whose last channel fails cannot publish until it verifies one again.

The seven-day window protects a vendor only if somebody hears about the release inside it, so a channel
that reaches a person is part of the right to publish. Adding a device needs a passkey, and every other
channel hears about it, so an attacker cannot quietly add a device of their own to receive the codes.

**Memory:** the service holds one message per send while it encrypts and posts it; subscriptions,
webhooks and one short row per event are on disk. A pending list checks the newest 500 releases no
owner confirmed, so a request's work stays bounded as the registry grows.

**Declined:**
- **Email.** The registry holds no address (`rule:packaging/a-registry-account-is-a-passkey`), and a
  mailbox is a takeover path.
- **Notification as an option an owner may skip.** An owner nobody can reach makes the pending window
  worthless for that vendor.
- **An Atom or RSS feed.** A feed reader cannot check a signature, and a second key only for feeds is one
  more secret for the operator. The log's key already signs, and its public key is already published.
- **Sending inside the change's transaction.** A slow push service would hold the database's write lock.
