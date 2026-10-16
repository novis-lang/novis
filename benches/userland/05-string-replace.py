# Substitution over a template - the placeholder fill every mailer and view helper does.


def run(rounds: int) -> int:
    template = "Hello {name}, your order {order} ships to {city} on {date}."
    names = []
    k = 0
    while k < 16:
        names.append("ab" * (k + 1))
        k = k + 1

    total = 1
    i = 0
    while i < rounds:
        line = template.replace("{name}", names[total % 16])
        line = line.replace("{order}", "A-1000")
        line = line.replace("{city}", "Vienna")
        line = line.replace("{date}", "2026-08-25")
        total = total + len(line)
        i = i + 1
    return total


print(run(200000))
