// Reads a JSON order, checks it, and returns its totals in cents as JSON.
// This measures reading, checking and writing JSON in a typical API request.

const isObject = (v) => typeof v === "object" && v !== null && !Array.isArray(v);
const isInt = (v) => Number.isSafeInteger(v);

// Returns the receipt for one decoded order, or null when the order fails a check.
function total(order) {
  if (!isObject(order) || !isObject(order.customer) || !Array.isArray(order.items)) return null;
  const { customer, items } = order;
  if (!isInt(customer.id) || typeof customer.name !== "string" || typeof customer.email !== "string") return null;
  if (customer.id < 1 || items.length < 1 || items.length > 100) return null;
  const lines = [];
  let subtotal = 0;
  for (const item of items) {
    if (!isObject(item) || typeof item.sku !== "string" || typeof item.name !== "string" || !isInt(item.qty) || !isInt(item.unitCents)) return null;
    if (item.sku === "" || item.qty < 1 || item.qty > 1000 || item.unitCents < 1 || item.unitCents > 10_000_000) return null;
    const total = item.qty * item.unitCents;
    lines.push({ sku: item.sku, qty: item.qty, totalCents: total });
    subtotal += total;
  }

  // The tax is 20 percent, rounded to the nearest cent. Half a cent rounds up.
  const tax = Math.floor((subtotal * 20 + 50) / 100);

  // The largest line comes first. `sort` is stable, so lines with the same total keep the order of the request.
  lines.sort((a, b) => b.totalCents - a.totalCents);

  return { customer: customer.id, items: lines, subtotalCents: subtotal, taxCents: tax, totalCents: subtotal + tax };
}

function json(res, status, value) {
  res.writeHead(status, { "Content-Type": "application/json" });
  res.end(JSON.stringify(value));
}

export default async function (req, res) {
  if (req.method !== "POST" || new URL(req.url, "http://localhost").pathname !== "/orders") {
    res.writeHead(404, { "Content-Type": "text/plain; charset=UTF-8" });
    res.end("not found\n");
    return;
  }
  const chunks = [];
  for await (const chunk of req) chunks.push(chunk);
  let order;
  try {
    // `TextDecoder` reads the body as UTF-8 and removes a byte order mark, as `fetch` does.
    order = JSON.parse(new TextDecoder().decode(Buffer.concat(chunks)));
  } catch {
    json(res, 422, { error: "invalid order" });
    return;
  }
  const receipt = total(order);
  if (receipt === null) json(res, 422, { error: "invalid order" });
  else json(res, 200, receipt);
}
