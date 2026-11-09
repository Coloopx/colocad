// Compile with tsc --noEmit --module NodeNext --target ES2022 --skipLibCheck.
import type { QueryRequest, Selection, Model } from '../../kernel/dist/index.js';
const selection: Selection={kind:'siblings',input:{kind:'entities',entity:'face'},entity:'face',via:'edge'};
const base={schema:'colocad.query/1',query_revision_id:'sha256:test',target:{definition_id:'part'},space:'definition' as const,tolerance:{linear_mm:1e-5,angular_rad:1e-7}};
const good:QueryRequest={...base,queries:[{id:'faces',op:'select',selection},{id:'named',op:'named_datums',ids:['mount']},{id:'material',op:'faces',where:{surface_type:'plane'}}]};
// @ts-expect-error A flattened operation still requires the query ID.
const missingId:QueryRequest={...base,queries:[{op:'faces'}]};
// @ts-expect-error Arbitrary filter classes must be rejected by generated types.
const invalidClass:QueryRequest={...base,queries:[{id:'bad',op:'faces',where:{surface_type:'arbitrary'}}]};
// @ts-expect-error Adjacency requires the shared topology kind.
const missingSharedKind:Selection={kind:'siblings',input:selection,entity:'face'};
const datums:Model['datums']=[{id:'origin',definition:'part',tolerance:base.tolerance,derive:{kind:'design_point',point:[0,0,0]}}];
void [good,missingId,invalidClass,missingSharedKind,datums];
