#!/bin/python3

import random

n = 10000
keys = [x for x in range(n)]
random.shuffle(keys)

# insert data

lines = ""
for key in keys:
    lines += f"insert\n{key}:{key}\nprint stats\nreset stats\n"

with open("insert-data.txt", "wt") as file:
    file.write(lines)

# search data
lines = ""
for key in keys:
    lines += f"insert\n{key}:{key}\n"

lines += "reset stats\n"

for key in keys:
    lines += f"search\n{key}\nprint stats\nreset stats\n"

with open("search-data.txt", "wt") as file:
    file.write(lines)

# delete data

lines = ""
for key in keys:
    lines += f"insert\n{key}:{key}\n"

lines += "reset stats\n"

for key in keys:
    lines += f"remove\n{key}\nprint stats\nreset stats\n"

with open("delete-data.txt", "wt") as file:
    file.write(lines)
