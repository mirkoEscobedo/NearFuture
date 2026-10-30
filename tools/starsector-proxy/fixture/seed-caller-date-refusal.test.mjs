// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {writeFileSync,readFileSync} from 'node:fs';
import {callerDateHistoryFixture} from './seed-caller-date-test-support.mjs';
import {ref,value} from './seed-failed-history-test-support.mjs';
import {json} from './seed-test-support.mjs';
import {readBatchAuthorization} from '../batch-authorization.mjs';

/** @param {any} f */
function refused(f){
 const next=f.launcher();let error;try{readBatchAuthorization(next.source,{root:f.root,runID:next.runID});}catch(cause){error=cause;}
 const ps=f.admit();assert.ok(error,'JS must refuse');assert.equal(ps.status,1,ps.stderr);
}
/** @type {[string,string,(v:any)=>void][]} */
const cases=[
 ['an added input','driver-flow-result.private.json',v=>v.counts.actions=1],
 ['a seed claim','driver-flow-result.private.json',v=>v.claims.seed=true],
 ['reservation refund','driver-flow-result.private.json',v=>v.history.refunded=2],
 ['forged zero unsent history','driver-flow-result.private.json',v=>v.history.historicalUnsentReservations=0],
 ['renewed clock','driver-flow-result.private.json',v=>v.failure.clockRenewed=true],
 ['a successful caller','driver-metadata-calls.actual.private.json',v=>v.entries[0].raw.exit_code=0],
 ['post-failure appended input command','driver-metadata-calls.actual.private.json',v=>v.entries[0].args.cmd+=';Send-Input'],
 ['a corrected caller mislabeled as historical failure','driver-metadata-calls.actual.private.json',v=>v.entries[0].args.cmd=v.entries[0].args.cmd.replace('$geo=Get-Content -LiteralPath $g.geometry.path -Raw|ConvertFrom-Json;','$geo=Get-Content -LiteralPath $g.geometry.path -Raw|ConvertFrom-Json -DateKind String;')],
 ['different literal date failure','driver-flow-result.private.json',v=>v.failure.literalError='Other failure'],
 ['a stop packet with input','driver-stop-observation.private.json',v=>v.inputs=1],
 ['doctor caller normalization rewritten','driver-stop-observation.private.json',v=>v.actualVendorCalls[0].args={include_remediation:true}],
 ['native point outside client','B1-PLAY-client-point.private.json',v=>v.localPoint.x=2],
 ['unmeasured coordinate','B1-PLAY-client-point.private.json',v=>v.coordinate=[90,90]],
 ['stale historical geometry','B1-PLAY-client-geometry.actual.private.json',v=>v.observedAtUtc=new Date(Date.parse(v.observedAtUtc)-6000).toISOString()],
 ['wrong geometry process birth','B1-PLAY-client-geometry.actual.private.json',v=>v.creationTime='2000-01-01T00:00:00Z'],
 ['geometry input grant','B1-PLAY-client-geometry.actual.private.json',v=>v.inputGranted=true],
 ['geometry timeout','B1-PLAY-client-geometry-terminal.actual.private.json',v=>v.supervisorJSON.result.steps[0].timedOut=true],
 ['unreaped geometry child','B1-PLAY-client-geometry-terminal.actual.private.json',v=>v.terminalReaped=false],
 ['wrong geometry child command','B1-PLAY-client-geometry-terminal.actual.private.json',v=>v.childCommand.args.push('extra')],
 ['foreign geometry executable','B1-PLAY-client-geometry-terminal.actual.private.json',v=>v.childCommand.program='G:/foreign/pwsh.exe'],
 ['geometry work past provider deadline','B1-PLAY-client-geometry-terminal.actual.private.json',v=>v.completedAtMs=v.providerClock.deadlineMs],
 ['foreign frame source','B1-PLAY-source-before.private.json',v=>v.claim.sha256='0'.repeat(64)],
 ['a different root clock in source frame','B1-PLAY-source-before.private.json',v=>v.rootClock.effectiveDeadlineMs++],
 ['parent birth substitution','B1-PLAY-owned-snapshot.private.json',v=>v.controllerCreationTime='2000-01-01T00:00:00Z'],
 ['a remaining owned process','owned-terminal-pid-absence.actual.private.json',v=>v.rows[0].absent=false],
 ['an omitted owned PID','owned-terminal-pid-absence.actual.private.json',v=>v.rows.pop()],
 ['array geometry status','B1-PLAY-client-geometry-terminal.actual.private.json',v=>v.status=[0]],
 ['array caller failure code','driver-metadata-calls.actual.private.json',v=>v.entries[0].raw.exit_code=[1]],
 ['array claim boolean','driver-flow-result.private.json',v=>v.claims.seed=[false]],
 ['array process creation timestamp','B1-PLAY-owned-snapshot.private.json',v=>v.controllerCreationTime=[v.controllerCreationTime]],
 ['array process PID','B1-PLAY-owned-snapshot.private.json',v=>v.controllerPid=[v.controllerPid]],
 ['present null MCP error flag','call-001.raw.private.json',v=>v.isError=null],
 ['array MCP error flag','call-001.raw.private.json',v=>v.isError=[false]],
 ['string MCP error flag','call-001.raw.private.json',v=>v.isError='false'],
 ['array doctor success flag','call-001.raw.private.json',v=>v.structuredContent.ok=[true]],
 ['array policy consent flag','call-002.raw.private.json',v=>v.structuredContent.approval_required_for_all=[true]],
 ['array attempt source digest','call-001.attempt.private.json',v=>v.sourceSha256=[v.sourceSha256]],
 ['changed historical predecessor clock','campaign-postlaunch.private.json',v=>v.rootClock.deadlineMs++],
 ['missing source inventory pin','frozen-support-pins.private.json',v=>v.pins=v.pins.filter((/** @type {any} */ x)=>!x.path.endsWith('read-owned-client-geometry.ps1'))],
];
for(const [name,file,mutate]of cases)test('both history gates refuse '+name,()=>{
 const f=callerDateHistoryFixture({change:(n,v)=>{if(n===file)mutate(v);}});
 try{refused(f);}finally{f.close();}
});

// Repin malformed bytes in the new root closeout/grant without repairing the malformed record.
for(const shape of ['root array','nested duplicate','invalid UTF8'])test('both history gates refuse repinned '+shape,()=>{
 const f=callerDateHistoryFixture();
 try{
  const path=f.evidencePaths.result,text=readFileSync(path,'utf8');
  writeFileSync(path,shape==='root array'?'['+text+']':shape==='nested duplicate'?text.replace('"vendorCalls":4','"vendorCalls":4,"vendorCalls":4'):Buffer.concat([Buffer.from(text),Buffer.from([0xff])]));
  f.closeout.evidence.result=ref(path);json(f.closeoutPath,f.closeout);
  const grant=value(f.grantPath);grant.previous.result=ref(f.closeoutPath);json(f.grantPath,grant);
  for(const a of [f.plan,f.launchLease,f.phasePlan,f.phaseLease])a.historyAdmission=ref(f.grantPath);
  f.refreshLaunch();refused(f);
 }finally{f.close();}
});

test('the caller-date failure cannot masquerade as completed seed history',()=>{
 const f=callerDateHistoryFixture();
 try{
  f.closeout.kind='ACTUAL_DISPOSABLE_SEED_CREATION_CLOSEOUT';json(f.closeoutPath,f.closeout);
  const grant=value(f.grantPath);grant.previous.result=ref(f.closeoutPath);json(f.grantPath,grant);
  for(const a of [f.plan,f.launchLease,f.phasePlan,f.phaseLease])a.historyAdmission=ref(f.grantPath);
  f.refreshLaunch();refused(f);
 }finally{f.close();}
});
