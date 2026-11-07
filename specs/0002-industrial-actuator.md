# Industrial actuator reconstruction and CLI film export

## Input and scope

Reconstruct a complete HIWIN KK60 standard ballscrew stage: 5 mm lead, 300 mm rail, A1 normal block, no motor/cover/switch. Manufacturer catalog K99TE16-1906, printed pp.15–16,22 (PDF pp.18–19,25), downloaded from HIWIN on 2026-11-07. The manufacturer's table is the dimensional authority. This is a catalog-based reconstruction, not OEM manufacturing CAD. Unpublished bearings, rolling contacts, recirculation and material choices are assumptions.

## Acceptance

- K1: full assembly contains machined U rail, carriage, grooved shaft, support, motor bracket, coupling, end seals, fasteners and rolling elements; fewer than 129 features and 129 named occurrences.
- K2: 300 mm rail, 60 mm width, 33 mm carriage height, 370 mm total length, 210 mm travel; six Ø5.5 holes on 30 × 100 mm pitches, Ø9.5 × 4.7 counterbores. Four nominal M5 holes on a 30 × 30 pattern, 8 mm deep (threads represented by nominal cylinders).
- K3: native B-rep helical groove constructed by lofting circular cutters and repeating at 5 mm lead; documented approximation, not a cosmetic texture. STEP readback checks topology and material at groove/crest locations.
- K4: rebuild at travel −105, 0, +105 mm; carriage translation and screw rotation share a parameter relation, one turn per 5 mm. Check endpoint body bounds, not a playback-only motion claim. No claim of a solved contact simulation.
- K5: persist feature construction and assembly milestones as immutable models, receipts, exact STEP and preview meshes. Independently read back each STEP using OCCT.
- R1: `colocad render SESSION OUTPUT.mp4|OUTPUT.gif` starts export, checks dependencies, captures real saved meshes with four camera views, and encodes locally. Optional Playwright/Chromium and FFmpeg; no CAD rebuild or cloud service during export.
- R2: actual step times remain visible; review duration is explicitly edited and must not be described as real-time modeling or a build123d benchmark. Output metadata records frame count, size, fps, source hashes and SHA-256.
- R3: short real-session film test checks ffprobe dimensions/frame count and viewer errors; invalid options fail before touching output. Existing CLI and viewer regression checks pass.

## Evidence

Tests are added before the model and renderer. Generated artifacts and independent verification are linked from the example README after execution. The synthetic turbofan remains a kernel fixture, not this industrial showcase.

## Executed result

16 successful native revisions; final 52 features / 81 solids. Independent OCCT: all revisions valid, 64 material probes passed, max relative volume difference 0.002608%, selected rail/carriage overlap zero at all three tested positions. MP4 and GIF exported through the CLI; four-view fit is regression-tested. The full product passed Linux arm64 1 vCPU / 512 MiB / no-network execution. Source and limits: `examples/hiwin-kk60/README.md`; measured evidence: `showcase/hiwin-kk60/evidence.json`.
