/** Typed ergonomic helpers for the host. Agent transport remains declarative JSON. */
import type { Feature, Instance, Pose, V3, Representation } from './types.js';
export type HardwareKind = 'compute-board' | 'accelerator' | 'display' | 'camera' | 'mic' | 'speaker' | 'box' | 'battery' | 'button' | 'coil' | 'connector';
export declare function pose(translation?: V3, rotation_deg?: V3, visible?: boolean): Pose;
export declare function occurrence(id: string, input: string, normal: Pose, internal?: Pose, exploded?: Pose, metadata?: Record<string, unknown>, representation?: Representation): Instance;
export declare function hardwareEnvelope(id: string, kind: HardwareKind, size: [number, number, number], shape?: 'box' | 'cylinder', axis?: 'x' | 'y' | 'z'): Feature[];
export declare function envelope(reason: string): Representation;
export declare function simplified(reason: string, omitted: string[], affects: string[]): Representation;
export declare function blindBoss(id: string, outerDiameter: number, height: number, holeDiameter: number, holeDepth: number): Feature;
