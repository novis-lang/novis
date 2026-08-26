# Serialising a nested payload - what every API response ends with.
import json


def run(rounds: int) -> int:
    tags = ["billing", "eu", "priority"]
    total = 0
    i = 0
    while i < rounds:
        payload = {
            "id": i,
            "name": "order-" + str(i),
            "status": "open",
            "amount": 1999,
            "tags": tags,
        }
        # Compact separators: PHP and MWL emit no spaces, and this case measures the encoded length.
        total = total + len(json.dumps(payload, separators=(",", ":")))
        i = i + 1
    return total


print(run(200000))
