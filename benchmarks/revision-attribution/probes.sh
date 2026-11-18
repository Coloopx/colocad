#!/bin/sh
set -eu
# Run from the source checkout. WORK contains the extracted npm package,
# inputs/, the sequential capture/ and separate result files.
work=$(cd "$1" && pwd)
inputs=$(cd "$2" && pwd)
scripts=$(cd "$(dirname "$0")/.." && pwd)
vol=$(docker volume create)
trap 'docker volume rm "$vol" >/dev/null' EXIT
run() {
    docker run --rm --network none --cpuset-cpus 2 --cpus 1 --memory 512m --memory-swap 512m \
        -v "$work/package/kernel:/pkg/kernel:ro" -v "$work/package/src:/pkg/src:ro" \
        -v "$work/package/viewer:/pkg/viewer:ro" -v "$scripts:/pkg/benchmarks:ro" \
        -v "$inputs:/inputs:ro" -v "$work/capture:/input:ro" -v "$work:/results" \
        -v "$vol:/store" "$@"
}
case "$3" in
    allocations)
        run -e COLOCAD_BENCH_ALLOCATIONS=/results/v8-profile.json colocad-memory:20261111 \
            node /pkg/benchmarks/field-task/colocad.mjs /inputs /store 0 64 ;;
    lookup)
        run colocad-memory:20261111 python /pkg/benchmarks/revision-attribution/layout.py \
            write segments /input /store /results/lookup-write.json
        run colocad-memory:20261111 /opt/cq/bin/python /pkg/benchmarks/revision-attribution/lookup.py \
            /store /pkg /inputs /results/cold-queries.json ;;
    *) echo 'Choose allocations or lookup' >&2; exit 2 ;;
esac
