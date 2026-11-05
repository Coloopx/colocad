export { CadClient, createCadTools } from "../kernel/dist/index.js";
export type Dim = number | { param: string };
export type Point = [Dim, Dim];
export type Profile =
  | { kind: "rectangle"; center: Point; size: Point }
  | { kind: "circle"; center: Point; radius: Dim }
  | { kind: "polygon"; points: Point[] };
export type Feature =
  | {
      id: string;
      op: "extrude" | "cut";
      profile: Profile;
      startZ?: Dim;
      depth: Dim;
    }
  | { id: string; op: "fillet"; edges: { z: Dim; count: Dim }; radius: Dim }
  | {
      id: string;
      op: "linearPattern";
      seed: string;
      axis: "x" | "y";
      pitch: Dim;
      count: Dim;
    };
export interface FeatureTree {
  format: "colocad/1";
  units: "mm";
  parameters: Record<string, number>;
  features: Feature[];
}
export interface Geometry {
  solids: number;
  volume_mm3: number;
  bbox_mm: number[][];
  edges: number;
  faces: number;
}
export function compileTree(tree: FeatureTree): Feature[];
export function kernelModel(
  tree: FeatureTree,
): import("../kernel/dist/types.js").Model;
export function build(
  tree: FeatureTree,
  destination: string,
  options?: { preview?: boolean; binary?: string },
): Promise<{
  directory: string;
  step: string;
  nativeJob: string;
  geometry: Geometry;
}>;
export function writeSolidWorks(
  input:
    | FeatureTree
    | {
        format: "colocad/native-job/1";
        features: Feature[];
        expected: { volumeMM3: number; bboxMM: number[][] };
      },
  destination: string,
): Promise<{
  file: string;
  reopened: boolean;
  editRebuildVerified: boolean;
  volumeMM3: number;
  nativeFeatureCount: number;
}>;
