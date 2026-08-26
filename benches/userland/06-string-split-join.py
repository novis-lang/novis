# split/join round trips - how userland parses a CSV line, a path or a header.


def run(rounds: int) -> int:
    lines = []
    k = 0
    while k < 16:
        lines.append("id" + str(k) + ",name,email,city,country,created,updated,status,score,notes")
        k = k + 1

    total = 1
    i = 0
    while i < rounds:
        fields = lines[total % 16].split(",")
        back = "|".join(fields)
        total = total + len(fields) + len(back)
        i = i + 1
    return total


print(run(200000))
