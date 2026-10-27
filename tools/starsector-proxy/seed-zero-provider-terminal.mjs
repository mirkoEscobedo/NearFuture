// @ts-check
import {resolve,isAbsolute} from 'node:path';
import {isDeepStrictEqual as same} from 'node:util';
import {closed} from './campaign-proof.mjs';

/** Strict object decoding for this custody boundary, including nested duplicate
 * keys that JSON.parse alone would overwrite. @param {string} text @param {(v:any)=>void} need */
export function zeroProviderObject(text,need){
 const result=JSON.parse(text);need(result!==null&&typeof result==='object'&&!Array.isArray(result));
 const tokens=text.match(/"(?:\\.|[^"\\])*"|[{}\[\]:,]|-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?|true|false|null/g)??[];let i=0;
 function value(){
  const token=tokens[i++];
  if(token==='{'){
   const keys=new Set();if(tokens[i]==='}'){i++;return;}
   do{const key=JSON.parse(tokens[i++]);need(!keys.has(key));keys.add(key);i++;value();}while(tokens[i++ ]===',');
  }else if(token==='['){if(tokens[i]===']'){i++;return;}do{value();}while(tokens[i++ ]===',');}
 }
 value();need(i===tokens.length);return result;
}

/** Actual tool streams are retained in order, including empty chunks. @param {any} first @param {any[]} polls @param {number} status @param {(v:any)=>void} need */
export function zeroProviderStream(first,polls,status,need){
 need(Array.isArray(polls)&&polls.length<=256);const rows=[first,...polls];let text='';
 const session=first?.session_id;
 for(const [i,r] of rows.entries()){
  need(typeof r?.output==='string'&&Number.isFinite(r.wall_time_seconds)&&r.wall_time_seconds>=0);
  need(Buffer.byteLength(r.output)<=65536);text+=r.output;need(Buffer.byteLength(text)<=131072);
  if(i===rows.length-1)need(r.exit_code===status&&!Object.hasOwn(r,'session_id'));
  else need(Number.isSafeInteger(session)&&session>0&&r.session_id===session&&r.exit_code==null);
 }
 return text;
}
/** @param {any} x */
const returnedStep=x=>({status:x.status,timedOut:x.timedOut,processOwnership:x.processOwnership,stdout:x.stdout,stderr:x.stderr});
/** Historical terminal custody; never invoke a child or reapply a current deadline. @param {any} context */
export function assertZeroProviderTerminal({v,a,e,need,powershell}){
 const {rootCall,caller,launch,combined:x,frozen:f,freezeSupervisor:z,plan:p,controller:c}=a;
 need(closed(x,['launch','stage','stagePolls','stop','stopPolls','launchPolls','error','b1Admit','remainingStageSession','remainingLaunchSession'])&&
  closed(x.stage,['args','raw'])&&closed(x.error,['name','message'])&&closed(z,['args','raw','polls','priorRejectedCall']));
 need(closed(caller,['schemaVersion','kind','session','launchRaw','launchPolls','actual','terminalReaped'])&&caller.schemaVersion===1&&
  caller.kind==='ACTUAL_ORIGINAL_LAUNCH_CALLER_TERMINAL'&&caller.session===rootCall.value.raw.session_id&&caller.terminalReaped===true&&
  same(caller.launchRaw,rootCall.value.raw)&&caller.launchPolls.length>=1&&same(caller.actual,caller.launchPolls.at(-1)));
 const returned=zeroProviderObject(zeroProviderStream(caller.launchRaw,caller.launchPolls,1,need),need),step=launch.result?.steps?.[0];
 need(['command','cwd','stdout','stderr'].every(k=>typeof step?.[k]==='string'));
 need(launch.schemaVersion===1&&launch.command==='verify'&&launch.ok===true&&launch.result.verdict==='FAIL'&&launch.result.steps.length===1&&
  step.status===1&&step.timedOut===false&&step.processOwnership==='windows-job-object'&&step.command===powershell&&step.cwd==='.'&&
  step.stderr.includes('Driver requested owned cleanup; native success not established')&&returned.verdict==='FAIL'&&same(returned.blocked,[])&&
  (!Array.isArray(returned.steps)||returned.steps.length===1)&&same(Array.isArray(returned.steps)?returned.steps[0]:returned.steps,returnedStep(step)));
 need(same(step.args,['-NoProfile','-NonInteractive','-File',resolve(v.sourceWorkspace,'tools/starsector-proxy/fixture/native-controller.ps1'),
  '-EvidenceRoot',v.scenario,'-BindingPath',p.fixtureBinding.path,'-BindingSha256',p.fixtureBinding.sha256,
  '-OuterDeadlineUtc',c.conservativeOuterDeadlineUtc,'-SessionWatchdogMs','480000']));
 need(x.error?.name==='Error'&&x.error.message==='Actual B1 stager not known terminal exit0; no provider'&&x.b1Admit===null&&
  x.remainingStageSession===null&&x.remainingLaunchSession===null&&same(x.launch,rootCall)&&same(x.launchPolls,caller.launchPolls)&&
  Number.isSafeInteger(x.stage?.raw?.session_id)&&x.stage.raw.session_id>0&&x.stage.raw.session_id!==caller.session);
 const stageOutput=zeroProviderStream(x.stage.raw,x.stagePolls,1,need);
 need(stageOutput.includes('Error: Campaign authority refused: actual controller clock and identity required')&&
  stageOutput.includes('Actual unclaimed schema6 B1 admission refused; no retry'));
 zeroProviderStream(x.stop,x.stopPolls,0,need);
 const q=(/** @type {any} */ value)=>"'"+String(value).replaceAll("'","''")+"'";
 const callerScript=p.preclockHelpers?.['launch-owned-caller.private.ps1'],stager=p.preclockHelpers?.['record-root-call-and-stage-B1.private.ps1'];
 need(callerScript?.path===resolve(v.scenario,'launch-owned-caller.private.ps1')&&stager?.path===resolve(v.scenario,'record-root-call-and-stage-B1.private.ps1'));
 const args=rootCall.value.args;
 need(args.workdir===v.sourceWorkspace&&args.cmd==='& '+q(powershell)+' -NoProfile -NonInteractive -File '+q(callerScript.path)+
  ' -PlanSha256 '+q(e.plan.sha256)+' -LeaseSha256 '+q(e.lease.sha256)+' -DelegationSha256 '+q(f.delegationSha256)+
  ' -HelperSha256 '+q(f.helperSha256)+' -HistoryAdmissionSha256 '+q(f.historyAdmissionSha256));
 need(x.stage.args.workdir===v.sourceWorkspace&&x.stage.args.cmd==='& '+q(powershell)+' -NoProfile -NonInteractive -File '+q(stager.path)+
  ' -RootCallJson '+q(JSON.stringify(rootCall))+' -HelperSha256 '+q(f.helperSha256)+' -HistoryAdmissionSha256 '+q(f.historyAdmissionSha256)+
  ' -DelegationSha256 '+q(f.delegationSha256));
 const frozenReport=zeroProviderObject(zeroProviderStream(z.raw,z.polls,0,need),need),fs=frozenReport.result?.steps?.[0];
 const freeze=p.preclockHelpers['root-freeze-and-preflight.seed.proposed.private.ps1'],loaded=p.source.loadedFrontendWitness;
 need(Array.isArray(fs?.args)&&fs.args.length===13&&fs.args.every((/** @type {any} */ x)=>typeof x==='string')&&isAbsolute(fs.args[3])&&isAbsolute(fs.args[10]));
 const freezeArgs=[...fs.args];freezeArgs[3]=resolve(freezeArgs[3]);freezeArgs[10]=resolve(freezeArgs[10]);
 need(freeze?.path===resolve(v.scenario,'root-freeze-and-preflight.seed.proposed.private.ps1')&&fs?.cwd==='.'&&
  same(freezeArgs,['-NoProfile','-NonInteractive','-File',freeze.path,'-RootAdmitted','-ProcedureSha256',freeze.sha256,
   '-PreclockIndexSha256',e.preclockIndex.sha256,'-LoadedFrontendWitnessPath',loaded.path,'-LoadedFrontendWitnessSha256',loaded.sha256]));
 need(frozenReport.schemaVersion===1&&frozenReport.command==='verify'&&frozenReport.ok===true&&frozenReport.result.verdict==='PASS'&&
  frozenReport.result.steps.length===1&&fs.status===0&&fs.timedOut===false&&fs.processOwnership==='windows-job-object'&&
  typeof fs.command==='string'&&isAbsolute(fs.command)&&resolve(fs.command)===powershell&&same(zeroProviderObject(fs.stdout,need),f));
 need(f.schemaVersion===1&&f.issuer==='/root'&&f.actualPsAdmissionExit===0&&f.actualWatchdogMs===480000&&
  f.planSha256===e.plan.sha256&&f.leaseSha256===e.lease.sha256&&f.historyAdmissionSha256===p.historyAdmission.sha256&&
  f.callerAdmissionDeadlineMs===p.callerAdmissionDeadlineMs&&f.cleanupStartNoLaterMs===p.callerAdmissionDeadlineMs-30000&&
  f.gameStarted===false&&f.providerStarted===false&&same(f.actualPsAdmissionOutput,[])&&
  f.helperSha256===p.preclockHelpers['construct-actual-phase-inputs.private.mjs'].sha256);
}
