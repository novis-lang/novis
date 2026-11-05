// The browser side of the chat. It uses the `WebSocket` class that every browser has,
// so the page needs no library.
const main = document.querySelector('main[data-socket]')
const list = document.getElementById('lines')
const form = document.getElementById('say')
const status = document.getElementById('status')

let socket = null
// The id of the newest line on the page. A line with this id or a smaller one is already shown.
let lastId = 0
// How long to wait before the next try to connect, in milliseconds.
let wait = 1000

// Opens the WebSocket. The address is the room page's `data-socket`, with `ws:` or `wss:`.
function connect() {
  const url = new URL(main.dataset.socket, location.href)
  url.protocol = location.protocol === 'https:' ? 'wss:' : 'ws:'
  socket = new WebSocket(url)
  socket.onopen = () => {
    status.textContent = 'Connected.'
    wait = 1000
  }
  socket.onmessage = (event) => show(JSON.parse(event.data))
  // When the connection closes, the page tries again. The wait doubles each time, up to 30 seconds.
  socket.onclose = () => {
    status.textContent = 'The connection is lost. Connecting again …'
    setTimeout(connect, wait)
    wait = Math.min(wait * 2, 30000)
  }
}

// Shows one message from the server. `kind` says what the message is.
function show(message) {
  if (message.kind === 'history') message.lines.forEach(addLine)
  if (message.kind === 'line') addLine(message.line)
  if (message.kind === 'joined') addNotice(`${message.name} joined the room.`)
  if (message.kind === 'left') addNotice(`${message.name} left the room.`)
  if (message.kind === 'problem') addNotice(message.text)
}

// Adds one chat line to the list. `append` with a string adds it as text, so HTML in a line is never run.
function addLine(line) {
  if (line.id <= lastId) return
  lastId = line.id
  const time = document.createElement('time')
  time.textContent = new Date(line.at).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
  const name = document.createElement('b')
  name.textContent = line.name
  const item = document.createElement('li')
  item.append(time, ' ', name, ': ', line.text)
  addItem(item)
}

function addNotice(text) {
  const item = document.createElement('li')
  item.className = 'notice'
  item.textContent = text
  addItem(item)
}

function addItem(item) {
  list.append(item)
  list.scrollTop = list.scrollHeight
}

// Sends the text of the form as one chat line.
form.addEventListener('submit', (event) => {
  event.preventDefault()
  const text = form.elements.text.value.trim()
  if (text === '' || socket.readyState !== WebSocket.OPEN) return
  socket.send(JSON.stringify({ kind: 'say', text }))
  form.elements.text.value = ''
})

// The server closes a connection with no message for 5 minutes.
// A `ping` message each minute keeps the connection open while the page is open.
setInterval(() => {
  if (socket.readyState === WebSocket.OPEN) socket.send(JSON.stringify({ kind: 'ping' }))
}, 60000)

connect()
