// The service worker of the registry. It shows each push message of your account.
// A test message has a code, and this sends the code back, so the device counts.
'use strict';

// Sends a `POST` request to the registry with the session cookie.
function post(path, body) {
  const options = { method: 'POST', credentials: 'same-origin', headers: {} };
  if (body !== undefined) {
    options.headers['content-type'] = 'application/json';
    options.body = JSON.stringify(body);
  }
  return fetch(path, options);
}

self.addEventListener('push', (event) => {
  if (!event.data) {
    return;
  }
  const message = event.data.json();
  const work = [
    self.registration.showNotification(message.title, {
      body: message.body,
      data: message,
      actions: message.here ? [{ action: 'here', title: 'Yes, I am here' }] : [],
    }),
  ];
  if (message.code) {
    work.push(post('/api/channels/' + encodeURIComponent(message.channel) + '/verify', { code: message.code }));
  }
  // A browser that does not allow notifications still sends the code.
  event.waitUntil(Promise.allSettled(work));
});

// The button "Yes, I am here" sends the answer of a reminder. A click on the message opens its page.
self.addEventListener('notificationclick', (event) => {
  event.notification.close();
  const message = event.notification.data || {};
  if (!message.here) {
    event.waitUntil(self.clients.openWindow('/account'));
    return;
  }
  const path = '/here/' + encodeURIComponent(message.here);
  event.waitUntil(event.action === 'here' ? post(path) : self.clients.openWindow(path));
});
