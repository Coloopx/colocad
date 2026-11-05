#!/bin/sh
# Usage: sh bench/run.sh IMAGE /absolute/file.tgz /absolute/NEW_OUTPUT_DIRECTORY
set -eu
base=$1
archive=$2
out=$3
root=$(CDPATH='' cd -- "$(dirname "$0")/../../.." && pwd)
hand="$root/showcase/tendon-hand"
mkdir "$out"
ctx=$(mktemp -d)
trap 'rm -rf "$ctx"' EXIT
cp "$archive" "$ctx/bundle.tgz"
cp "$root/skills/colocad/scripts/assembly.mjs" "$hand/bench/baseline.py" "$hand/bench/window.py" "$ctx/"
node "$hand/model.mjs" "$ctx/inputs"
cat > "$ctx/Dockerfile" <<'EOF'
ARG BASE
FROM ${BASE}
ADD bundle.tgz /opt/
COPY baseline.py assembly.mjs window.py /bench/
COPY inputs/drive.json /inputs/
EOF
img=$(docker build -q --network none --build-arg "BASE=$base" "$ctx")
docker image inspect "$base" --format '{{.Id}}' > "$out/base-image.txt"
for arm in build123d colocad; do
 docker run --rm --network none --cpus 1 --memory 512m --memory-swap 512m --read-only --pids-limit 64 --cap-drop ALL --security-opt no-new-privileges --user 1000:1000 --tmpfs /tmp:rw,nosuid,size=128m -v "$out:/outputs" -e RAYON_NUM_THREADS=1 -e OMP_NUM_THREADS=1 -e OPENBLAS_NUM_THREADS=1 -e MKL_NUM_THREADS=1 -e NUMEXPR_NUM_THREADS=1 -e XDG_CACHE_HOME=/tmp -e MPLCONFIGDIR=/tmp/mpl -e PYTHONDONTWRITEBYTECODE=1 -e "BASELINE_PYTHON=${BASELINE_PYTHON:-/home/agent/workspace/.structure-runtime/venv/bin/python3}" --entrypoint python3 "$img" /bench/window.py "$arm" 100
done
