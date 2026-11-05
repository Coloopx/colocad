export function pose(translation = [0, 0, 0], rotation_deg = [0, 0, 0], visible = true) { return { translation, rotation_deg, rotation_order: 'zyx', visible }; }
export function occurrence(id, input, normal, internal = normal, exploded = normal, metadata = {}, representation = { kind: 'exact' }) { return { id, input, poses: { normal, internal, exploded }, metadata, representation }; }
export function hardwareEnvelope(id, kind, size, shape = 'box', axis = 'z') {
    if (!['compute-board', 'accelerator', 'display', 'camera', 'mic', 'speaker', 'box', 'battery', 'button', 'coil', 'connector'].includes(kind) || !size.every(n => Number.isFinite(n) && n > 0))
        throw Error('invalid hardware envelope');
    const [x, y, z] = size;
    if (shape === 'box')
        return [{ op: 'box', id, origin: [-x / 2, -y / 2, 0], size }];
    const at = { x: 0, y: 1, z: 2 }[axis], cross = size.filter((_, i) => i !== at);
    if (Math.abs(cross[0] - cross[1]) > 1e-6)
        throw Error('cylinder perpendicular sizes must be equal');
    const source = `${id}-stock`;
    return [{ op: 'cylinder', id: source, origin: [0, 0, 0], radius: cross[0] / 2, height: size[at] }, { op: 'place', id, input: source, translation: axis === 'x' ? [-x / 2, 0, z / 2] : axis === 'y' ? [0, -y / 2, z / 2] : [0, 0, 0], rotation_deg: axis === 'x' ? [0, 90, 0] : axis === 'y' ? [-90, 0, 0] : [0, 0, 0] }];
}
export function envelope(reason) { if (!reason.trim())
    throw Error('envelope reason required'); return { kind: 'envelope', reason }; }
export function simplified(reason, omitted, affects) { if (!reason.trim() || !omitted.length || !affects.length)
    throw Error('simplification declaration incomplete'); return { kind: 'simplified', reason, omitted, affects }; }
export function blindBoss(id, outerDiameter, height, holeDiameter, holeDepth) {
    if (![outerDiameter, height, holeDiameter, holeDepth].every(n => Number.isFinite(n) && n > 0) || holeDiameter >= outerDiameter || holeDepth >= height)
        throw Error('boss requires a smaller blind pilot hole');
    return { id, op: 'boss', origin: [0, 0, 0], outer_radius: outerDiameter / 2, height, hole_radius: holeDiameter / 2, hole_depth: holeDepth };
}
