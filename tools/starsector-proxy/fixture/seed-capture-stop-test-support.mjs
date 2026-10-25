// @ts-check
// Generated canonical capture-stop boundary data. No private game evidence.
import {knownPlayHistoryFixture} from './seed-known-play-history-test-support.mjs';
import {json} from './seed-test-support.mjs';
import {value} from './seed-failed-history-test-support.mjs';
import assert from 'node:assert/strict';
import {readBatchAuthorization} from '../batch-authorization.mjs';

export function captureStopHistoryFixture(){
 const f=knownPlayHistoryFixture({targetApp:'java.exe',historyBaseline:{overallAttempts:43,uiAttempts:40,reservedActions:73,sentActions:71}});
 const paths=f.evidencePaths,args=structuredClone(f.knownPlay.pending.args);
 const r=value(paths.result),o=value(paths.observation);
 Object.assign(r,{failureCategory:'CAPTURE_UNCERTAIN',assertionResult:'UNKNOWN',helperModesExecuted:0,
  actions:{reservedThisRun:1,sentThisRun:1,clicks:1,activations:0,stage:'PLAY',args,
   replacementWindow:20,ownedPid:101,status:'AWAITING_AFTER',uncertain:false},
  originals:{count:3,allInspected:true,launcherNativeWidth:2,gameNativeWidth:2,
   capture001:'Launcher Play visible',capture002:'Loading bar',capture003:'Dark mainmenu transition; no readable controls',
   capture003PngSha256:o.originals[2].png.sha256,freshness:'UNKNOWN',captureReceiptDistinctFromSuccess:true,
   completeMainmenuVerified:false,permittedLoadingRecapturesUsed:1},
  seed:{created:false,modernCloseoutCreated:false,stabilityModeExecuted:false,semanticAcceptance:'NOT_RUN'}});
 json(paths.result,r);
 const assertions=['Launcher Play visible','Starsector loading bar','Dark fading mainmenu shell; no readable control labels'];
 for(const key of ['guardHistory','visibleOutcome','helperRefusal','callerError'])delete o[key];
 Object.assign(o,{kind:'ACTUAL_DRIVER_KNOWN_PLAY_CAPTURE_STOP',category:'POSTCONDITION_UNKNOWN_CAPTURE_UNCERTAIN',
  reason:'Generated unreadable final permitted original; no next input',actualGuardHistory:{...f.knownPlay.guard},
  actions:{clicks:1,activations:0,args:structuredClone(args),pid:101,replacementWindow:20,
   knownInput:true,ledgerStatus:'AWAITING_AFTER',uncertain:false},
  originals:o.originals.map((/** @type {any} */ row,/** @type {number} */ i)=>({...row,assertion:assertions[i]})),
  permittedLoadingRecapturesUsed:1,helperModesExecuted:0});
 json(paths.observation,o);
 f.closeout.kind='ROOT_CLOSED_CAPTURE_STOP_FAILED_SEED_ATTEMPT_HISTORY';f.refresh();
 return f;
}

/** @param {any} f @param {string} key @param {(v:any)=>void} change */
export function editCaptureEvidence(f,key,change){
 const path=f.evidencePaths[key],v=value(path);change(v);json(path,v);
}

/** Public admission refuses after all changed metadata has been coherently repinned.
 * @param {any} f */
export function assertCaptureRefused(f){
 const b=f.launcher();let jsError;
 try{readBatchAuthorization(b.source,{root:f.root,runID:b.runID});}catch(error){jsError=error;}
 const launch=f.admit();assert.equal(launch.error,undefined);assert.notEqual(launch.status,null);
 assert.ok(jsError instanceof Error,'real JS reader must refuse');
 assert.match(jsError.message,/seed|history|custody|capture|previous/i);
 assert.notEqual(launch.status,0);assert.match(launch.stderr,/seed|history|custody|capture|previous/i);
}
