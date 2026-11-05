# How many complete hand revisions fit in 100 seconds?

The optimized colocad kernel saved **44** complete tendon-hand STEP assemblies. build123d 0.11.1 saved **33** under the same 100-second budget. Each hand contains 43 solids. For all 33 shared revisions the colocad call finished in less time. [10× MP4](hand-window-en.mp4) · [Single-file player with downloads](https://github.com/Coloopx/colocad/raw/refs/heads/main/showcase/tendon-hand/window/hand-window.html) · [Raw event times, memory samples and readbacks](../evidence/window-optimized.json)

| One serial 100 s window per backend | Optimized colocad | build123d |
| :--- | ---: | ---: |
| Complete STEP assemblies at the deadline | **44** | 33 |
| Time to first handoff | **2.223 s** | 3.010 s |
| Median job time on the same first 33 inputs | **2.234 s** | 2.855 s |
| Highest sampled cgroup memory inside the 100 s window | **130.9 MiB** | 304.9 MiB |
| Last counted handoff | 99.43 s | 97.23 s |
| Next handoff, outside the window | 101.58 s | 100.19 s |

**Where the comparison applies.** colocad delivered 33.3% more complete hand revisions and used 42.9% of build123d's sampled peak container memory on this queue. Instantaneous memory depends on the phase of the call. At 42.4 s, for example, the most recent samples were 74.3 MiB for colocad and 62.0 MiB for build123d.

**How to read the video.** Two separately recorded windows share a zero point and a 0–512 MiB vertical scale. A tick appears only after an entire STEP is handed off to persistent storage. The right-hand rows show the latest completed revision and its guide diameter measured from the saved STEP. The four reference views, machining and editing the finger link, the mesh and the assembly sequence on the left are a walkthrough of the common model. The counters and both memory readouts stay visible during every shot. Each of the 250 frames represents the end of a 0.4 s measurement interval. Playback lasts 10.00 s.

**Measurement.** The test ran in preinstalled Linux arm64 containers with 1 vCPU, 512 MiB, no swap, a read-only root and no network. Each arm completed one excluded warm-up. Every job then started a fresh process in the same container, built the same ordered queue of unique parameters, assembled the hand, ran its native checks, exported STEP and handed off the file. Bookkeeping and scratch cleanup between jobs consumed window time. Inference, container launch, preparing the film and independent STEP readback occurred outside timing. The colocad arm used the public alpha.2 SDK with the source-optimized binary (SHA-256 recorded in the environment entry). This is one actual window per backend.

**Synchronized memory.** A sampler in each container read cgroup v2 `memory.current` approximately every 25 ms, producing 3,893 colocad samples and 3,663 build123d samples. Values include the timer and sampling process, CAD processes, tmpfs and charged file cache pages, including those accumulating during a sequence of exports. The line holds a reading until the next actual sample. Peak is the largest sample observed from the window start up to the displayed time. Shorter spikes between samples can be missed. The separately published per-job `memory.peak` statistic uses a fresh cgroup for each call. [Separate single-call benchmark](../assembly-performance.md)

**Verification.** Every counted STEP reopened in OCCT as a valid assembly of 43 solids. The smaller queue was extended off-clock to compare every one of the 44 unique inputs. Across all pairs the maximum per-solid bounds difference was below 1.1e-7 mm and the maximum relative volume difference below 7e-9. Each revision’s index-finger bore and cord radius were measured from both actual exports. Both directions of the boolean difference measured zero for all 20 unique definitions of the first hand.

To reproduce, run from the checkout with the [optimized Linux arm64 kernel](../assembly-performance.md) and an installed build123d evaluation environment. Supply absolute paths to the archive, output directory and kernel.

```sh
sh showcase/tendon-hand/bench/run.sh abelien/runner:dev "$PWD/coloopx-colocad-0.1.0-alpha.2.tgz" "$PWD/window-run" "$PWD/kernel/native/linux-arm64/agent-cad"
node showcase/tendon-hand/model.mjs ./models
.venv-eval/bin/python showcase/tendon-hand/bench/verify.py ./window-run ./models/drive.json ./consumer ./window.json --native-arm optimized
```

The consumer directory is used only if a missing native counterpart must be rebuilt off-clock. In that case also pass `--supplemental-kernel` with a compatible optimized host executable. The [public alternative sandbox image](../../../benchmarks/readme/Dockerfile.public) accepts `BASELINE_PYTHON=/usr/local/bin/python3`. For the animation pass the verified JSON, the geometric stills from [assets.py](../tools/assets.py), and a new media directory to [window-film.py](../tools/window-film.py).
