// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {callerDateHistoryFixture} from './seed-caller-date-test-support.mjs';
import {readBatchAuthorization} from '../batch-authorization.mjs';

/** @param {string} file @param {(v:any)=>void} mutate */
const change=(file,mutate)=>({change:(/** @type {string} */ name,/** @type {any} */ v)=>{if(name===file)mutate(v);}});
/** @type {[string,Parameters<typeof callerDateHistoryFixture>[0]][]} */
const cases=[
 ['same JSON support aliases with different bytes',{aliasBytes:v=>Buffer.concat([v,Buffer.from('\n')])}],
 ['plan reference to wrong support alias',change('plan.json',v=>v.source.supportPins.path=v.source.supportPins.path.replace('source-pins.private.json','frozen-support-pins.private.json'))],
 ['invented initial controller fields',change('driver-B1-owned.private.json',v=>v.controllerPid=100)],
 ['foreign initial source alias',change('driver-B1-owned.private.json',v=>v.sourceSnapshot.path+='-foreign')],
 ['array initial source digest',change('driver-B1-owned.private.json',v=>v.sourceSnapshot.sha256=[v.sourceSnapshot.sha256])],
 ['phase cap differs from original lease',change('B1-lease.private.json',v=>v.absoluteMaxReservedActions=85)],
 ['phase watchdog differs',change('B1-lease.private.json',v=>v.sessionWatchdogMs=480001)],
 ['phase history changes',change('B1-lease.private.json',v=>v.historyAdmission.sha256='0'.repeat(64))],
 ['phase batches differ',change('B1-plan.private.json',v=>v.batches=[])],
 ['foreign coherent campaign flow',{campaign:v=>v.flow='FOREIGN_FLOW'}],
 ['foreign coherent Java parent',change('owned-processes-current.private.json',v=>v.processes[0].parentPid=999)],
 ['self parent',change('owned-processes-current.private.json',v=>v.processes[0].parentPid=v.processes[0].pid)],
 ['observed Java missing from owned claim',change('guard-B1-consumed.private.json',v=>v.binding.ownedPids=[100])],
 ['Java born after initial observation',change('owned-processes-current.private.json',v=>v.processes[0].creationTime=new Date(Date.parse(v.processes[0].creationTime)+200).toISOString())],
 ['array native window handle',change('B1-PLAY-owned-snapshot.private.json',v=>v.processes[0].mainWindowHandle=[19])],
 ['array image MIME',{capture:v=>v.content[0].mimeType=['image/png']}],
 ['array image bytes',{capture:v=>v.content[0].data=[v.content[0].data]}],
 ...['rawPath','rawSha256','pngPath','pngSha256','freshness'].map(key=>/** @type {[string,Parameters<typeof callerDateHistoryFixture>[0]]} */(['array metadata '+key,{metadata:v=>v[key]=[v[key]]}])),
 ['object provider steps',{provider:v=>v.steps=v.steps[0]}],
 ['array provider command',{provider:v=>v.steps[0].command=[v.steps[0].command]}],
 ['array provider ownership',{provider:v=>v.steps[0].processOwnership=[v.steps[0].processOwnership]}],
 ['array provider cwd',{provider:v=>v.steps[0].cwd=[v.steps[0].cwd]}],
 ['array spool command',{spool:v=>v.command=['verify']}],
 ['array spool verdict',{spool:v=>v.result.verdict=['PASS']}],
 ['array returned verdict',change('launch-caller-terminal.actual.private.json',v=>{const out=JSON.parse(v.actual.output);out.verdict=['FAIL'];v.actual.output=JSON.stringify(out);v.launchPolls.at(-1).output=v.actual.output;})],
 ['array root driver',change('plan.json',v=>v.ownership.driver=['/root/recovery_game_driver'])],
 ['duplicate finish embedded verdict',change('driver-stop-observation.private.json',v=>v.providerFinish.raw.content[0].text=v.providerFinish.raw.content[0].text.replace('"verdict":"PASS"','"verdict":"FAIL","verdict":"PASS"'))],
 ['duplicate begin embedded runID',change('driver-stop-observation.private.json',v=>v.begin.raw.content[0].text=v.begin.raw.content[0].text.replace('"runID":','"runID":"different","runID":'))],
 ['begin MCP error',change('driver-stop-observation.private.json',v=>v.begin.raw.isError=true)],
 ['finish MCP error',change('driver-stop-observation.private.json',v=>v.providerFinish.raw.isError=true)],
 ['begin MCP null error flag',change('driver-stop-observation.private.json',v=>v.begin.raw.isError=null)],
 ['finish MCP array error flag',change('driver-stop-observation.private.json',v=>v.providerFinish.raw.isError=[false])],
];
for(const [name,options]of cases)test('R1 both public readers refuse '+name,()=>{
 const f=callerDateHistoryFixture(options);
 try{
  const next=f.launcher();let error;try{readBatchAuthorization(next.source,{root:f.root,runID:next.runID});}catch(cause){error=cause;}
  const ps=f.admit();assert.ok(error,'JS must refuse coherent '+name);assert.equal(ps.status,1,'PS must refuse coherent '+name+'\n'+ps.stderr);
 }finally{f.close();}
});
