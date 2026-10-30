// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {knownPlayHistoryFixture} from './seed-known-play-history-test-support.mjs';
import {ref,value} from './seed-failed-history-test-support.mjs';
import {json} from './seed-test-support.mjs';
import {readBatchAuthorization} from '../batch-authorization.mjs';

/** Repin every changed boundary record, so refusal cannot rely on an obsolete digest.
 * @param {any} f @param {{screenshot?:[string,any],raw?:[number,(v:any)=>void],bounds?:(v:any)=>void,ledger?:(v:any)=>void,observation?:(v:any)=>void,metadata?:[string,any],image?:[string,any]}} change */
function mutate(f,change){
 const o=value(f.evidencePaths.observation);
 if(change.screenshot){
  const [key,replacement]=change.screenshot;
  for(const which of ['post','attempt']){const path=o.calls[13][which].path,v=value(path);v.args[key]=replacement;json(path,v);o.calls[13][which]=ref(path);}
  const path=o.originals[2].metadata.path,m=value(path);m.args[key]=replacement;json(path,m);o.originals[2].metadata=ref(path);
 }
 if(change.raw){const [index,apply]=change.raw,path=o.calls[index].raw.path,v=value(path);apply(v);json(path,v);o.calls[index].raw=ref(path);}
 if(change.ledger){const path=f.evidencePaths.ledger,v=value(path);change.ledger(v);json(path,v);}
 if(change.observation)change.observation(o);
 if(change.metadata){const path=o.originals[2].metadata.path,v=value(path);v[change.metadata[0]]=change.metadata[1];json(path,v);o.originals[2].metadata=ref(path);}
 if(change.image){
  const frame=o.originals[2],raw=value(frame.raw.path);raw.content[0][change.image[0]]=change.image[1];
  json(frame.raw.path,raw);json(o.calls[13].raw.path,raw);
  const m=value(frame.metadata.path);m.rawSha256=ref(frame.raw.path).sha256;json(frame.metadata.path,m);
  frame.raw=ref(frame.raw.path);frame.metadata=ref(frame.metadata.path);o.calls[13].raw=ref(o.calls[13].raw.path);
 }
 if(change.bounds){
  for(const index of [2,5,9,12]){const path=o.calls[index].raw.path,v=value(path);change.bounds(v.structuredContent.bounds);json(path,v);o.calls[index].raw=ref(path);}
  const listPath=o.calls[8].raw.path,list=value(listPath);change.bounds(list.structuredContent.windows[0].bounds);json(listPath,list);o.calls[8].raw=ref(listPath);
  for(const row of o.originals){const path=row.metadata.path,v=value(path);change.bounds(v.window.bounds);json(path,v);row.metadata=ref(path);}
 }
 json(f.evidencePaths.observation,o);f.refresh();
}
/** @type {[string,Parameters<typeof mutate>[1]][]} */
const cases=[
 ['boolean screenshot quality',{screenshot:['quality',false]}],
 ['boolean screenshot width',{screenshot:['width',true]}],
 ['boolean screenshot target',{screenshot:['target_window_id',true]}],
 ['boolean frontmost PID',{raw:[11,v=>{v.structuredContent.app.pid=true;}]}],
 ['boolean window PID',{raw:[5,v=>{v.structuredContent.pid=true;}]}],
 ['boolean window ID',{raw:[5,v=>{v.structuredContent.windowId=true;}]}],
 ['boolean list PID',{raw:[8,v=>{v.structuredContent.windows[0].pid=true;}]}],
 ['boolean list window ID',{raw:[8,v=>{v.structuredContent.windows[0].windowId=true;}]}],
 ['boolean app identifier',{raw:[5,v=>{v.structuredContent.bundleId=true;}]}],
 ['boolean x geometry',{bounds:v=>{v.x=false;}}],
 ['string x geometry',{bounds:v=>{v.x='0';}}]
 ,['boolean pending tool',{ledger:v=>{v.pending.tool=true;}}]
 ,['boolean pending status',{ledger:v=>{v.pending.status=true;}}]
 ,['boolean freshness label',{observation:v=>{v.originals[2].freshness=true;}}]
 ,['boolean metadata run ID',{metadata:['runID',true]}]
 ,['numeric raw error flag',{raw:[11,v=>{v.isError=0;}]}]
 ,['string raw error flag',{raw:[11,v=>{v.isError='false';}]}]
 ,['boolean source-call path',{observation:v=>{v.calls[13].post.path=true;}}]
 ,['boolean image type',{image:['type',true]}]
 ,['boolean image MIME type',{image:['mimeType',true]}]
 ,['boolean image data',{image:['data',true]}]
];
for(const [name,change] of cases)test('known Play typed custody refuses '+name+' in both readers',()=>{
 const f=knownPlayHistoryFixture();
 try{
  mutate(f,change);const b=f.launcher();let jsError;
  try{readBatchAuthorization(b.source,{root:f.root,runID:b.runID});}catch(error){jsError=error;}
  const ps=f.admit();
  assert.ok(jsError,'JavaScript admitted malformed '+name);
  assert.notEqual(ps.status,0,'PowerShell admitted malformed '+name);
 }finally{f.close();}
});

test('known Play typed custody refuses a boolean explicit target app',()=>{
 const f=knownPlayHistoryFixture(/** @type {any} */({targetApp:true}));
 try{
  const b=f.launcher();let jsError;try{readBatchAuthorization(b.source,{root:f.root,runID:b.runID});}catch(error){jsError=error;}
  const ps=f.admit();assert.ok(jsError);assert.notEqual(ps.status,0);
 }finally{f.close();}
});

test('known Play typed custody refuses a coherently rewritten boolean session ID',()=>{
 const f=knownPlayHistoryFixture({sessionId:true});
 try{
  const b=f.launcher();let jsError;try{readBatchAuthorization(b.source,{root:f.root,runID:b.runID});}catch(error){jsError=error;}
  const ps=f.admit();assert.ok(jsError);assert.notEqual(ps.status,0);
 }finally{f.close();}
});

test('known Play typed geometry retains valid negative desktop coordinates',()=>{
 const f=knownPlayHistoryFixture({originX:-10,originY:-10});
 try{
  const b=f.launcher();assert.doesNotThrow(()=>readBatchAuthorization(b.source,{root:f.root,runID:b.runID}));
  const result=f.admit();assert.equal(result.status,0,result.stderr);
 }finally{f.close();}
});
