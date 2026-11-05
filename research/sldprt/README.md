# Headless native encoding probe

From the repository root, build a part with the public CLI. Then in a fresh scratch directory, run the following with `repo` set to the absolute source checkout. The envelope generator writes only framing data and a blank document marker. It consumes no input part. The format tokens were observed together in the public [OpenWorkstation sample](https://github.com/SebastianEggert/OpenWorkstation_hardware/blob/d0669b64a01a182b8d977d9daba7d86911306e9d/moduleStorage/m-st_holder-suction-cups_v1.1.SLDPRT).

```sh
cargo build --manifest-path "$repo/research/sldprt/Cargo.toml"
probe="$repo/research/sldprt/target/debug/colocad-native-probe"
python3 "$repo/research/sldprt/blank_envelope.py"
"$probe" read-step "$repo/out/manifold/part/model.step" > from-step.json
python3 "$repo/research/sldprt/equivalent_ranges.py" from-step.json equivalent.json
"$probe" probe equivalent.json empty-envelope.bin
```

The normalizer verifies each carrier's parameter domain and endpoint agreement before using vertex-delimited edges. It rejects partial spline intervals, partial circle arcs and unrecognized carriers. The experiment measures B-rep recovery independently of instruction compilation. STEP supplies geometry only. A recoverable kernel body cannot establish that SolidWorks received editable feature inputs.
