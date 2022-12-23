#!/bin/bash

SIZES=(84 126 168 210)

for size in ${SIZES[@]}
do
    cat "insert-output-${size}.txt" | tail -n +6 | awk  '{ if ($1 == "Index:") print $3, $5 }' > "insert-result-${size}.txt"
    cat "search-output-${size}.txt" | tail -n +6 | awk  '{ if ($1 == "Index:") print $3, $5 }' > "search-result-${size}.txt"
    cat "delete-output-${size}.txt" | tail -n +6 | awk  '{ if ($1 == "Index:") print $3, $5 }' > "delete-result-${size}.txt"
    cat "insert-output-${size}.txt" | tail -n +6 | awk  '{ if ($1 == "Index:") print $7 }' > "size-result-${size}.txt"
done
