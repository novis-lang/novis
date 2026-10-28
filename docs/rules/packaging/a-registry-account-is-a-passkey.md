A registry account is one or more passkeys and a set of one-time recovery codes shown once at sign-up,
with **no email, no SMS and no password**, and it is bound to no forge.

- **Sign-up** carries a small proof-of-work against bulk sign-ups. An account can do nothing until its
  first publish, and a publish needs a public git repository
  (`rule:packaging/the-registry-builds-the-archive-from-git`).
- **Several passkeys** may be registered on one account. Signing in is a WebAuthn ceremony verified by
  `Core\Passkey` (`rule:core-classes/passkey-verifies-a-webauthn-ceremony`).
- **There is no human recovery.** Losing every passkey and every recovery code loses the account. Its
  packages keep working, and its co-owners keep publishing; the guide advises two owners per vendor.
- **Contact with an owner** is the inbox on the website and the package's public repository.

Every way an account is usually taken over goes through something this design does not have. A
password is phished or reused; a mailbox is taken over and its reset link followed; an SMS code is
swapped to another SIM; a forge account is compromised and its OAuth grant inherited. A passkey is bound
to the registry's origin and cannot be typed into a fake page. Recovery by a human is the step attackers
talk their way through, so there is none, and the cost of that is carried by a second owner rather
than by a support desk.

**Declined:**
- **Email accounts, with reset links.** A mailbox is the most common takeover path, and an address is
  personal data the registry would have to hold.
- **Sign-in through GitHub, GitLab or another forge, or OIDC trusted publishing.** It makes one forge's
  account system part of every package's security and shuts out every other git host.
- **Passwords with a second factor.** A password is still phishable, and the second factor's usual
  fallback is SMS or email.
- **Account recovery by the operator.** A process a person can be talked through is the weakest
  credential on the account.

**On disk.** `apps/registry/src/App.nvs` is the sign-up, the sign-in and the passkey routes;
`Work.nvs` is the proof-of-work, a SHA-256 of a session-bound challenge with as many leading zero bits
as `hosting.json`'s `signUpWork` names, checked once and within ten minutes; `Accounts.nvs` keeps ten
recovery codes per account as SHA-256 hashes, each used once. The schema has no column for a name, an
address or a password. **Memory:** one challenge per visitor in the session, and no state between
requests outside SQLite.
