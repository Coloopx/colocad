# Quick construction in millimetres

Run from the project containing the installed npm dependency. Set `skill` to the directory containing this skill's `SKILL.md`. For a 100 × 50 × 6 mm plate with a centered 3 × 2 pattern of Ø6 mm through-holes, use the following command. The pitches are center-to-center, 32 mm along X and 30 mm along Y.

```sh
skill="${CODEX_HOME:-$HOME/.codex}/skills/colocad"
node "$skill/scripts/build.mjs" "$skill/assets/plate-grid.json" ./plate-v1 \
  --set length=100 --set xCount=3 --set xPitch=32 --preview
```

Each invocation takes a new output directory. Edit the saved `model.json` to add features or pass it back to the helper with another `--set` to change an existing dimension. Overrides are saved in the built model. Only request a preview when you need to inspect it.

| Shape or structure | Shortest construction in this skill | Datum and acceptance check |
|---|---|---|
| Block, pin, ball or taper | Use one feature from [block](../assets/block.json), [pin](../assets/pin.json), [ball](../assets/ball.json) or [taper](../assets/taper.json). For example, pass `pin.json` with `--set diameter=8 --set length=40` for a Ø8 × 40 mm pin. | Box origin is the lower corner. A cylinder or cone starts at its base center and grows along +Z. Check the outer envelope. |
| Mounting plate or perforated rail | [Plate with centered hole grid](../assets/plate-grid.json) uses a box and one `face_hole_grid`. Use `yCount=1` for one row. Keep both pitches positive. | XY starts at the lower corner. Confirm count, hole centers and minimum edge material. For centered columns the end margin is `(length − (xCount−1) × xPitch − holeDiameter) / 2`. Check Y likewise. |
| Ring, spacer or plain sleeve | [Sleeve](../assets/sleeve.json) extrudes an annular profile in one feature. Use positive diameters with the bore smaller than the outside diameter. | Axis is +Z, bottom at Z=0. Check that the bore goes through and the radial wall is `(outerDiameter−boreDiameter)/2`. |
| Right-angle mounting bracket | [L-section with perpendicular holes](../assets/angle-bracket.json) extrudes one L profile, then drills each leg from its outside face. Shorten the feature list and select `angle` as output for a plain angle. | Section lies in XZ and extrudes toward −Y. The foot and upright dimensions include the shared corner. Check one connected solid and both bore axes. |
| Open housing with a side window | [Housing](../assets/open-housing.json) uses `shell_body` followed by `face_cutout`. Set the output to `shell` and remove the last feature for an uncut tray. | XY centered, floor starts at Z=0, opening is +Z. Port center is on +Y at world (X,Z). Check wall, floor and air through the side port, including material above and below it. |

For a custom single solid, append a native feature referencing the previous feature's ID and set `output` to the new ID. A `face_*` operation starts on the selected outer bounding plane and cuts inward. Centers on ±X use world (Y,Z), on ±Y use (X,Z), and on ±Z use (X,Y). For a recessed or inclined face use a local sketch `frame` and `holes` or `cutout`. A diameter from the drawing becomes a radius in the native operation. Keep the conversion as a parameter expression so edits rebuild correctly.

## Place the same part twice

Given two identical 16 mm sleeves separated by a 4 mm axial gap, start from the sleeve model. This uses the same definition for both occurrences and ties the upper position to the edited length. Run in the same shell as above.

```sh
node --input-type=module - "$skill/assets/sleeve.json" > pair.json <<'JS'
import { readFileSync } from 'node:fs';
const model = JSON.parse(readFileSync(process.argv[2], 'utf8'));
const sleeve = model.output;
delete model.output;
model.parameters.gap = 4;
model.assembly = [
  { id: 'lower', input: sleeve, translation: [0, 0, 0] },
  { id: 'upper', input: sleeve, translation: [0, 0,
    { op: 'add', a: { param: 'length' }, b: { param: 'gap' } }] }
];
console.log(JSON.stringify(model, null, 2));
JS
node "$skill/scripts/build.mjs" pair.json ./pair-v1 --set length=20
```

Check two solids on the same axis. In the edited model the lower sleeve spans Z=0…20 and the upper spans Z=24…44 mm. For one fused part, combine the stock and additions with native features and select one `output`. For an assembly, name each physical occurrence and preserve independent solids.
