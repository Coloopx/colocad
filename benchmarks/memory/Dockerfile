FROM node:22-bookworm-slim AS node
FROM python:3.12-slim-bookworm
COPY --from=node /usr/local/bin/node /usr/local/bin/node
RUN apt-get update && apt-get install -y --no-install-recommends libgl1 libglu1-mesa libxrender1 libxext6 libsm6 libgomp1 && rm -rf /var/lib/apt/lists/*
RUN python -m venv /opt/bd && /opt/bd/bin/pip install --no-cache-dir build123d==0.13.0
RUN python -m venv /opt/cq && /opt/cq/bin/pip install --no-cache-dir cadquery==2.8.0
ENV OMP_NUM_THREADS=1 OPENBLAS_NUM_THREADS=1 MKL_NUM_THREADS=1 NUMEXPR_NUM_THREADS=1 RAYON_NUM_THREADS=1 PYTHONDONTWRITEBYTECODE=1
WORKDIR /bench
