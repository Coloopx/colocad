# Synthetic turbofan geometry fixture

This is an original illustrative layout, not the reconstruction of a real engine model. The assistant chose the dimensions and wrote `model.mjs`. No engineering drawing was converted to this model. Its purpose is exercising geometry, assembly, persistence and the viewer; it does not validate multimodal reconstruction or industrial design fidelity.

From the repository root:

```sh
npm ci
npm run build:viewer
node examples/turbofan/run.mjs ./out/turbofan
node bin/colocad.mjs view ./out/turbofan/session --cutaway
node bin/colocad.mjs replay ./out/turbofan/session ./out/turbofan/review.html
```

The generator emits 16 real modeling milestones and a final 98-solid assembly. It includes a bored fan hub, lofted blades, repeated blade rows, concentric shafts, a perforated combustor, revolved casings and a nozzle. The exposed parameters are fan radius, combustor port radius and shaft radius; the remaining dimensions are generator constants.

`run.mjs` resumes completed labels in its output directory. Use a new directory after changing the generator so old completed stages are not mistaken for new results. History retains actual call timings and native feature receipts. It is not an inference benchmark or a production engine specification.

Independent development validation (requires `cadquery-ocp`):

```sh
python examples/turbofan/verify.py ./out/turbofan/session ./out/turbofan/verification.json
```

The recorded development run reopened all 16 STEP files, checked per-solid validity and counts, compared aggregate volume to the native receipt, and checked the hub bore and eight combustor ports. Maximum observed relative volume discrepancy was about 0.00362%, under the verifier's 0.01% bound. No aerodynamic, stress, tolerance-stack or mechanism-clearance certification is claimed.

Structural background: [NASA turbofan overview](https://www.grc.nasa.gov/WWW/K-12/airplane/aturbf.html). This source supplies the component arrangement, not the fixture's dimensions. The user-facing product reconstruction task must use a specified real model and source package; see the [workflow distinction](../../docs/modeling-workflow.zh-CN.md).
