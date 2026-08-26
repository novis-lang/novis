# Building output one piece at a time - the shape of every hand-rolled template and CSV writer.
# Python's idiom for this is a list and one join, where PHP's is `.=`; the README's fairness rule
# is that each side is written the way that language writes it.


def run(rows: int) -> str:
    parts = []
    i = 0
    while i < rows:
        parts.append("<tr><td>" + str(i) + "</td><td>row-" + str(i) + "</td></tr>")
        i = i + 1
    return "".join(parts)


html = run(20000)
print(len(html))
