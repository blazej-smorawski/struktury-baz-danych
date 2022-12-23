#!/bin/bash

SIZES=(84 126 168 210)

# for size in ${SIZES[@]}
# do
#     OUTPUT=$(./target/release/proj-2 -i $size < ./insert-data.txt)
#     echo "$OUTPUT" > "insert-output-${size}.txt"
# done

# for size in ${SIZES[@]}
# do
#     OUTPUT=$(./target/release/proj-2 -i $size < ./search-data.txt)
#     echo "$OUTPUT" > "search-output-${size}.txt"
# done

for size in ${SIZES[@]}
do
    OUTPUT=$(./target/release/proj-2 -i $size < ./delete-data.txt)
    echo "$OUTPUT" > "delete-output-${size}.txt"
done
