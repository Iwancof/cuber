with open("./raw_nbt.txt", "r") as f:
    content = f.read()

content = content.replace('  ', ' ').replace('  ', ' ').replace('  ', ' ').replace('  ', ' ').replace('  ', ' ').replace('  ', ' ')
bin = []

for line in content.split('\n'):
    val = list(map(lambda x: int(x, 16), line.split(' ')[: -1]))
    bin.extend(val)


with open("binary", "wb") as f:
    f.write(bytes(bin))

