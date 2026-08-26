# String-keyed insert and read back - the cache, the index, the lookup table.


def run(entries: int) -> int:
    index = {}
    i = 0
    while i < entries:
        index["key-" + str(i)] = i
        i = i + 1

    total = 0
    i = 0
    while i < entries:
        key = "key-" + str(i * 3 % entries)
        if key in index:
            total = total + index[key]
        i = i + 1
    return total


print(run(200000))
