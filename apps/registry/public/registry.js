// The script of every page of the registry. It runs each passkey step and sends each form.
// A form with `data-action` names its step below. The registry checks every step again.
'use strict';

// Sends a request with the session cookie. It returns the status and the JSON answer.
async function send(method, path, body) {
  const options = { method, credentials: 'same-origin', headers: {} };
  if (body !== undefined) {
    options.headers['content-type'] = 'application/json';
    options.body = typeof body === 'string' ? body : JSON.stringify(body);
  }
  const response = await fetch(path, options);
  const text = await response.text();
  let answer = {};
  try {
    answer = text === '' ? {} : JSON.parse(text);
  } catch {
    answer = {};
  }
  return { status: response.status, answer };
}

// Throws the error the registry gave when the status is not 2xx.
function check(result) {
  if (result.status < 200 || result.status > 299) {
    throw new Error(result.answer.error || 'The registry did not accept the request. Try again.');
  }
  return result.answer;
}

// Writes a message in the status line at the top of the page.
function say(text) {
  const status = document.getElementById('status');
  if (status) {
    status.textContent = text;
  }
}

// Counts the zero bits at the start of a hash.
function zeroBits(bytes) {
  let bits = 0;
  for (const byte of bytes) {
    if (byte === 0) {
      bits += 8;
      continue;
    }
    return bits + Math.clz32(byte) - 24;
  }
  return bits;
}

// Finds a number whose SHA-256 of `challenge:number` starts with `bits` zero bits.
async function solve(challenge, bits) {
  const encoder = new TextEncoder();
  for (let nonce = 0; ; nonce++) {
    const hash = await crypto.subtle.digest('SHA-256', encoder.encode(challenge + ':' + nonce));
    if (zeroBits(new Uint8Array(hash)) >= bits) {
      return String(nonce);
    }
  }
}

// Makes a new passkey on this device with the options of the registry.
async function createPasskey(options) {
  const credential = await navigator.credentials.create({
    publicKey: PublicKeyCredential.parseCreationOptionsFromJSON(options),
  });
  return JSON.stringify(credential.toJSON());
}

// Signs in with a passkey of this device. It returns `true` when the registry accepts it.
async function signInWithPasskey() {
  const options = check(await send('POST', '/api/signin/options'));
  const credential = await navigator.credentials.get({
    publicKey: PublicKeyCredential.parseRequestOptionsFromJSON(options),
  });
  check(await send('POST', '/api/signin', JSON.stringify(credential.toJSON())));
}

// Sends a step that needs a passkey from the last 15 minutes. When the registry answers `403`,
// this signs in with a passkey first and sends the step one more time.
async function withPasskey(form, step) {
  const result = await step();
  if (result.status !== 403 || !form.hasAttribute('data-passkey')) {
    return check(result);
  }
  say('Sign in with your passkey to continue.');
  await signInWithPasskey();
  return check(await step());
}

// Shows a value the registry gives only once, such as a new token.
function showOnce(section, field, value) {
  document.getElementById(field).textContent = value;
  document.getElementById(section).hidden = false;
}

// Subscribes this browser to push messages and adds it as a device of the account.
async function addDevice(form) {
  const registration = await navigator.serviceWorker.register('/push.js');
  await navigator.serviceWorker.ready;
  const key = check(await send('GET', '/api/push/key'));
  const subscription = await registration.pushManager.subscribe({
    userVisibleOnly: true,
    applicationServerKey: key.applicationServerKey,
  });
  const body = { subscription: JSON.stringify(subscription.toJSON()) };
  return withPasskey(form, () => send('POST', '/api/push/devices', body));
}

// Each step a form can name in `data-action`. A step returns a message, or `null` to reload the page.
const steps = {
  async signup() {
    say('Your browser is doing the calculation. This can take a few seconds.');
    const work = check(await send('POST', '/api/signup/work'));
    const nonce = await solve(work.challenge, work.bits);
    const options = check(await send('POST', '/api/signup/options', { challenge: work.challenge, nonce }));
    const account = check(await send('POST', '/api/signup', await createPasskey(options)));
    const list = document.getElementById('code-list');
    for (const code of account.recoveryCodes) {
      const item = document.createElement('li');
      item.textContent = code;
      list.append(item);
    }
    document.getElementById('codes').hidden = false;
    return 'Your account is ready. Its id is ' + account.account + '.';
  },
  async signin() {
    await signInWithPasskey();
    location.assign('/account');
    return 'You are signed in.';
  },
  async recovery(form, fields) {
    const answer = check(await send('POST', '/api/signin/recovery', { code: fields.get('code') }));
    location.assign('/account');
    return 'You are signed in. You have ' + answer.recoveryCodesLeft + ' recovery codes left.';
  },
  async signout() {
    check(await send('POST', '/api/signout'));
    location.assign('/');
    return 'You are signed out.';
  },
  async reminders(form, fields) {
    check(await send('POST', '/api/account/reminders', { days: Number(fields.get('days')) }));
    return null;
  },
  async 'add-passkey'() {
    const options = check(await send('POST', '/api/account/passkeys/options'));
    const added = check(await send('POST', '/api/account/passkeys', await createPasskey(options)));
    return 'The passkey is added. Your account has ' + added.passkeys + ' passkeys.';
  },
  async token(form, fields) {
    const body = { scope: fields.get('scope'), days: Number(fields.get('days')) };
    const made = await withPasskey(form, () => send('POST', '/api/tokens', body));
    showOnce('new-token', 'token-text', made.token);
    return 'The token is made. It works for ' + made.days + ' days.';
  },
  async 'revoke-token'(form) {
    check(await send('DELETE', '/api/tokens/' + encodeURIComponent(form.dataset.id)));
    return null;
  },
  async 'add-owner'(form, fields) {
    const path = '/api/vendors/' + encodeURIComponent(form.dataset.vendor) + '/owners';
    await withPasskey(form, () => send('POST', path, { account: fields.get('account') }));
    return null;
  },
  async 'remove-owner'(form) {
    const path = '/api/vendors/' + encodeURIComponent(form.dataset.vendor) + '/owners/' + encodeURIComponent(form.dataset.id);
    await withPasskey(form, () => send('DELETE', path));
    return null;
  },
  async 'add-device'(form) {
    await addDevice(form);
    return 'This browser is added. It counts once its test message arrives. Reload the page to see it.';
  },
  async 'add-webhook'(form, fields) {
    const added = await withPasskey(form, () => send('POST', '/api/webhooks', { url: fields.get('url') }));
    showOnce('new-webhook', 'webhook-secret', added.secret);
    return 'The webhook is added. Its test message has a code. Enter the code here to verify the webhook.';
  },
  async 'verify-channel'(form, fields) {
    check(await send('POST', '/api/channels/' + encodeURIComponent(form.dataset.id) + '/verify', { code: fields.get('code') }));
    return null;
  },
  async 'test-channel'(form) {
    check(await send('POST', '/api/channels/' + encodeURIComponent(form.dataset.id) + '/test'));
    return 'The test message is sent.';
  },
  async 'remove-channel'(form) {
    check(await send('DELETE', '/api/channels/' + encodeURIComponent(form.dataset.id)));
    return null;
  },
};

// Runs the step of a form when it is sent. The page reloads when the step returns `null`.
document.addEventListener('submit', async (event) => {
  const form = event.target;
  const step = steps[form.dataset.action];
  if (step === undefined) {
    return;
  }
  event.preventDefault();
  const button = form.querySelector('button');
  button.disabled = true;
  try {
    const message = await step(form, new FormData(form));
    if (message === null) {
      location.reload();
      return;
    }
    say(message);
  } catch (error) {
    say(error.message);
  } finally {
    button.disabled = false;
  }
});
