import { CadClient, type Model, type Receipt } from './client.js';
// Bind these tools to the agent host's function-calling or MCP transport.
// The host owns runner configuration and artifact paths; model input cannot choose them.
export async function createCadTools(client: CadClient) {
  const schema = await client.schema();
  if (!schema.ok) throw Error(schema.error.message);
  const modelSchema = schema.data;
  // JSON Schema references resolve from the tool root, so lift definitions with it.
  const { $defs, $schema, ...modelBody } = modelSchema;
  const object = (properties: Record<string, unknown>, required: string[]) => ({ type:'object', properties, required, additionalProperties:false, $defs });
  const tools = [
    { name:'cad_capabilities', description:'查询当前内核、操作、预算与未实现能力。', inputSchema:{type:'object',properties:{},additionalProperties:false}, execute:()=>client.capabilities() },
    { name:'cad_validate', description:'低成本检查参数、特征依赖和预算；不执行几何验证。', inputSchema:object({model:modelBody},['model']), execute:(a:{model:Model})=>client.validate(a.model) },
    { name:'cad_build', description:'生成新的不可变版本，执行几何检查，返回体积、尺寸、逐特征耗时与 STEP 工件目录。queryable 保存精确查询快照。默认失败明确报错；只有模型显式允许 on_fail:record 才记录降级。', inputSchema:object({model:modelBody,options:{type:'object',properties:{queryable:{type:'boolean'}},additionalProperties:false}},['model']), execute:(a:{model:Model;options?:import('./types.js').BuildOptions})=>client.build(a.model,a.options) },
    { name:'cad_preview', description:'输出 PNG、GLB 和 STL；相同模型、姿态和精度复用已校验网格，切换视角或图像尺寸不重建。改参按特征依赖复用检查点；section 可按世界坐标平面剖视。精确检查使用 B-rep。',inputSchema:object({model:modelBody,options:{$ref:'#/$defs/PreviewOptions'}},['model']),execute:(a:{model:Model;options?:import('./types.js').PreviewOptions})=>client.preview(a.model,a.options)},
    { name:'cad_solve_sketch',description:'求解参数草图，返回局部自由度、冗余方程及每条约束残差。未声明的欠约束和冲突会失败。',inputSchema:object({sketch:{$ref:'#/$defs/Sketch'},parameters:{type:'object',additionalProperties:{type:'number'}}},['sketch']),execute:(a:{sketch:import('./types.js').Sketch;parameters?:Record<string,number>})=>client.solveSketch(a.sketch,a.parameters)},
    { name:'cad_inspect_step',description:'解析已登记 STEP 的产品树、共享零件定义和变换，返回可重建模型模板。使用原 model.assets 构建模板；不缩放组件。',inputSchema:object({model:modelBody,asset:{type:'string'}},['model','asset']),execute:(a:{model:Model;asset:string})=>client.inspectStep(a.model,a.asset)},
    { name:'cad_edit', description:'修改已有尺寸参数，按依赖重新计算受影响的特征，返回新版本。', inputSchema:object({model:modelBody,parameters:{type:'object',additionalProperties:{type:'number'}}},['model','parameters']), execute:(a:{model:Model;parameters:Record<string,number>}):Promise<Receipt>=>client.edit(a.model,a.parameters) }
  ];
  const capabilities=await client.capabilities();
  if(capabilities.ok&&(capabilities.data.query as {schema?:string}|undefined)?.schema==='colocad.query/1') {
    const schema=await client.querySchema();if(!schema.ok)throw Error(schema.error.message);
    const {$defs:queryDefs,$schema:querySchema,...request}=schema.data;
    return [...tools,{name:'cad_query',description:'查询已保存的精确几何：有界选择、拓扑/基准、孔组/安装接口与整零件关系测量；保留歧义和 partial 诊断。必须提供预期 query_revision_id；不重建、不改模型。路径受宿主允许的工件目录约束。',inputSchema:{type:'object',properties:{artifact_directory:{type:'string'},request},required:['artifact_directory','request'],additionalProperties:false,$defs:queryDefs},execute:(a:{artifact_directory:string;request:import('./types.js').QueryRequest})=>client.query(a.artifact_directory,a.request)}];
  }
  return tools;
}
