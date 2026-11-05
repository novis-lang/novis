// Updates the results of a poll page while it is open. It uses the browser's own
// EventSource, so the page needs no library.
const form = document.getElementById('vote')
const total = document.getElementById('total')
const status = document.getElementById('status')

// EventSource opens the stream and connects again by itself when the connection breaks.
const source = new EventSource(form.dataset.events)

// The server sends a `results` event when the page connects, and again after each vote.
// The data is JSON, such as {"total": 3, "answers": [{"id": 1, "votes": 2}, {"id": 2, "votes": 1}]}.
source.addEventListener('results', (event) => {
  const results = JSON.parse(event.data)
  total.textContent = String(results.total)
  for (const answer of results.answers) {
    const item = form.querySelector(`[data-answer="${answer.id}"]`)
    if (item === null) continue
    const bar = item.querySelector('progress')
    bar.max = Math.max(results.total, 1)
    bar.value = answer.votes
    item.querySelector('.votes').textContent = String(answer.votes)
  }
})

source.addEventListener('open', () => {
  status.textContent = 'The results update live.'
})

source.addEventListener('error', () => {
  status.textContent = 'Connecting again...'
})

// A click on an answer sends the vote with fetch, so the page stays open.
// The new numbers then arrive as a `results` event, the same way they arrive for every other visitor.
form.addEventListener('submit', async (event) => {
  event.preventDefault()
  const chosen = event.submitter
  const body = new URLSearchParams({ answer: chosen.value })
  // After a vote, the server sends a redirect to the poll page. With `manual`, fetch does not load that page.
  // The response also sets the cookie `voter` the first time, and the browser keeps it.
  await fetch(form.action, { method: 'POST', body, redirect: 'manual' })
  // The class `chosen` moves to the answer this browser voted for.
  for (const button of form.querySelectorAll('button.chosen')) button.classList.remove('chosen')
  chosen.classList.add('chosen')
})
