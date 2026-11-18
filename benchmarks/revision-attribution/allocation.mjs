import {Session} from 'node:inspector/promises';
import {writeFile} from 'node:fs/promises';

const session=new Session();
export async function start() {
  session.connect();
  await session.post('HeapProfiler.startSampling',{
    samplingInterval:32768,includeObjectsCollectedByMajorGC:true,includeObjectsCollectedByMinorGC:true
  });
}
export async function finish(path) {
  const {profile}=await session.post('HeapProfiler.stopSampling');
  session.disconnect();
  await writeFile(path,JSON.stringify(profile));
}
