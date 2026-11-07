/** Browser-only custom element. Import registers <colocad-viewer>. */
export interface ViewerSession {
  format: 'colocad/viewer/1'; title: string; id: string; createdAt: string;
  display?: { cutaway?: boolean; view?: 'iso'|'front'|'side'|'top' };
  steps: Array<Record<string, any>>;
  meshes: Record<string, {positions: number[][]; normals?: number[][]; triangles: number[][]}>;
}
export class ColocadViewer extends HTMLElement {
  session: ViewerSession;
  setStep(index: number): void;
  setView(view: 'iso'|'front'|'side'|'top'): void;
  select(id: string): void;
  setProjection(mode: 'perspective'|'orthographic'): void;
  setSectionAxis(axis: 'x'|'y'|'z'): void;
  play(): void;
  pause(): void;
  getState(): {stepIndex: number; status?: string; parts: number; visibleParts: number; view: string; exploded: number; cutaway: boolean; playing: boolean; selectedId: string|null; webgl: boolean};
}
