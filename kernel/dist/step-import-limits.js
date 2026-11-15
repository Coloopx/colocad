const ceilings = { maxEntities: 2_000_000, maxDefinitions: 1024, maxOccurrences: 4096, maxGroups: 8192 };
export function stepImportArgs(value) {
    if (value === undefined)
        return [];
    if (value === null || typeof value !== 'object' || Array.isArray(value))
        throw Error('stepImportLimits must be an object');
    const snapshot = Object.create(null);
    for (const [key, limit] of Object.entries(value)) {
        const ceiling = Object.hasOwn(ceilings, key) ? ceilings[key] : undefined;
        if (!ceiling || !Number.isSafeInteger(limit) || limit < 1 || limit > ceiling)
            throw Error(`stepImportLimits.${key} must be an integer in 1..${ceiling ?? 'supported ceiling'}`);
        snapshot[key] = limit;
    }
    return ['--step-import-limits', JSON.stringify(snapshot)];
}
