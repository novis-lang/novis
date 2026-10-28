An account must show activity within 60 days, or it can no longer publish; **token use never counts as
activity**. The 60 days are fixed and are not a setting. The account's packages stay available, and its
releases are untouched.

**Activity is one of two things:**

- **A passkey sign-in** (`rule:packaging/a-registry-account-is-a-passkey`).
- **The "Yes, I am here" answer to a reminder.** A Web Push reminder carries a one-time code inside its
  encrypted payload, and the action button, or the notification itself through a small page, sends that
  code back. A webhook reminder carries a one-time "I am here" link that counts the same way, so an
  owner whose only channel is a webhook can answer too
  (`rule:packaging/an-owner-is-notified-without-email`).

**An answer keeps an active account active, and cannot revive a lapsed one.** After 60 days without
activity, only a passkey sign-in makes the account active again. The owner chooses when reminders start,
such as after 14, 30 or 45 quiet days.

A token lives in CI and is the credential most likely to leak, and an account kept alive by its own
token would stay able to publish long after its owner stopped looking. The rule ties publishing to a
person who answered recently. The answer to a reminder needs the device that holds the push
subscription or the channel the owner verified, which a leaked token does not give, and a lapsed
account needs the passkey itself.

**Declined:**
- **Counting a publish or any token use as activity.** A stolen token would then keep the account it
  was stolen from active forever.
- **A period the owner sets.** The 60 days are a bound on how long a leaked token stays useful, and a
  bound that each owner can raise is not one.

**On disk** in `apps/registry/src/Activity.nvs`. `accounts.active_at` is the last activity, and
`Publish` refuses an account whose `active_at` is 60 days old. The owner picks the first reminder
from 7 to 59 quiet days (30 until they choose, `POST /api/account/reminders`), and an account that
does not answer gets another every 7 days until it lapses. The daily job sends them through
`Notices::tell`. Each reminder has a new one-time code, stored as a hash, that works until the account
would lapse and only once. A push message carries it as `here` in its encrypted payload; a webhook
message is `{"type": "reminder", "link": "<origin>/here/<code>"}`. `GET /here/<code>` is the small
page with the "Yes, I am here" button, and only its `POST` counts, so a link preview that opens the
page counts nothing. **Memory:** one row per account that set a reminder or got one.
