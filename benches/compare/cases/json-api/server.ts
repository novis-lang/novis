// Reads a JSON order, checks it, and returns its totals in cents as JSON.
// This measures reading, checking and writing JSON in a typical API request.

interface Line {
  sku: string;
  qty: number;
  totalCents: number;
}

const isObject = (v: unknown): v is Record<string, unknown> => typeof v === "object" && v !== null && !Array.isArray(v);
const isInt = (v: unknown): v is number => Number.isSafeInteger(v);

// Returns the receipt for one decoded order, or null when the order fails a check.
function total(order: unknown) {
  if (!isObject(order) || !isObject(order.customer) || !Array.isArray(order.items)) return null;
  const { customer, items } = order;
  if (!isInt(customer.id) || typeof customer.name !== "string" || typeof customer.email !== "string") return null;
  if (customer.id < 1 || items.length < 1 || items.length > 100) return null;
  const lines: Line[] = [];
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

const json = (status: number, value: unknown) =>
  new Response(JSON.stringify(value), { status, headers: { "Content-Type": "application/json" } });

export default {
  async fetch(req: Request): Promise<Response> {
    if (req.method !== "POST" || new URL(req.url).pathname !== "/orders") {
      return new Response("not found\n", { status: 404, headers: { "Content-Type": "text/plain; charset=UTF-8" } });
    }
    let order: unknown;
    try {
      order = await req.json();
    } catch {
      return json(422, { error: "invalid order" });
    }
    const receipt = total(order);
    return receipt === null ? json(422, { error: "invalid order" }) : json(200, receipt);
  },
};
