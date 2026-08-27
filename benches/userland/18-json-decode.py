# Parsing then reading the fields - Novis validates into a declared type where Python hands back a dict.
import json


def run(rounds: int) -> int:
    payloads = []
    k = 0
    while k < 16:
        payloads.append(
            '{"id":' + str(4711 + k) + ',"name":"order-4711","status":"open","amount":1999}'
        )
        k = k + 1

    total = 1
    i = 0
    while i < rounds:
        order = json.loads(payloads[total % 16])
        total = (total + order["id"] + order["amount"] + len(order["name"])) % 1000003
        i = i + 1
    return total


print(run(200000))
