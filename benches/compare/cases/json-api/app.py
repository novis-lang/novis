# Reads a JSON order, checks it, and returns its totals in cents as JSON.
# This measures reading, checking and writing JSON in a typical API request.
import json

# The largest whole number a JSON number holds exactly in every peer.
MAX_INT = 2**53 - 1


def is_int(v):
    # `bool` is a kind of `int` in Python, so the type is compared exactly.
    return type(v) is int and -MAX_INT <= v <= MAX_INT


def reject_constant(name):
    # JSON has no `NaN` or `Infinity`, and `json.loads` reads them unless this function throws an error.
    raise ValueError(f"{name} is not JSON")


def total(order):
    """Returns the receipt for one decoded order, or None when the order fails a check."""
    if not isinstance(order, dict) or not isinstance(order.get("customer"), dict) or not isinstance(order.get("items"), list):
        return None
    customer, items = order["customer"], order["items"]
    if not is_int(customer.get("id")) or not isinstance(customer.get("name"), str) or not isinstance(customer.get("email"), str):
        return None
    if customer["id"] < 1 or len(items) < 1 or len(items) > 100:
        return None
    lines = []
    subtotal = 0
    for item in items:
        if not isinstance(item, dict) or not isinstance(item.get("sku"), str) or not isinstance(item.get("name"), str):
            return None
        if not is_int(item.get("qty")) or not is_int(item.get("unitCents")):
            return None
        if item["sku"] == "" or item["qty"] < 1 or item["qty"] > 1000 or item["unitCents"] < 1 or item["unitCents"] > 10_000_000:
            return None
        line_total = item["qty"] * item["unitCents"]
        lines.append({"sku": item["sku"], "qty": item["qty"], "totalCents": line_total})
        subtotal += line_total

    # The tax is 20 percent, rounded to the nearest cent. Half a cent rounds up.
    tax = (subtotal * 20 + 50) // 100

    # The largest line comes first. `sort` is stable, so lines with the same total keep the order of the request.
    lines.sort(key=lambda line: -line["totalCents"])

    return {"customer": customer["id"], "items": lines, "subtotalCents": subtotal, "taxCents": tax, "totalCents": subtotal + tax}


async def respond(send, status, content_type, body):
    await send({"type": "http.response.start", "status": status, "headers": [(b"content-type", content_type)]})
    await send({"type": "http.response.body", "body": body})


async def json_reply(send, status, value):
    body = json.dumps(value, separators=(",", ":"), ensure_ascii=False).encode()
    await respond(send, status, b"application/json", body)


async def app(scope, receive, send):
    if scope["method"] != "POST" or scope["path"] != "/orders":
        await respond(send, 404, b"text/plain; charset=UTF-8", b"not found\n")
        return
    chunks = []
    while True:
        message = await receive()
        chunks.append(message.get("body", b""))
        if not message.get("more_body", False):
            break
    try:
        # `json.loads` reads bytes as UTF-8 and removes a byte order mark.
        order = json.loads(b"".join(chunks), parse_constant=reject_constant)
    except (ValueError, RecursionError):
        await json_reply(send, 422, {"error": "invalid order"})
        return
    receipt = total(order)
    if receipt is None:
        await json_reply(send, 422, {"error": "invalid order"})
    else:
        await json_reply(send, 200, receipt)
