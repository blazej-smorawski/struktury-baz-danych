#/bin/bash

OUTPUT=$(./target/release/proj-1 -r 128)
SIZES=(4 8 16 32 64 128 256 512 1024 2048 4096 8192 16384)

for size in ${SIZES[@]}
do
    OUTPUT=$(./target/release/proj-1 -b 230 -r $size)
    echo "$OUTPUT" | tail -n3 | awk -F'[^0-9]*' '$0=$2' | tr '\n' '\t'
    echo $size
done
