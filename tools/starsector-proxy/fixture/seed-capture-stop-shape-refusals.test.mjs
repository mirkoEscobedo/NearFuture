// @ts-check
import test from 'node:test';
import {captureStopHistoryFixture,editCaptureEvidence,assertCaptureRefused} from './seed-capture-stop-test-support.mjs';

/** @type {[string,string,(v:any)=>void][]} */
const cases=[
 ['helper-refusal category','result',v=>{v.failureCategory='PRIVATE_FRAME_CUSTODY_REFUSAL_AFTER_KNOWN_PLAY';}],
 ['verified assertion','result',v=>{v.assertionResult='PASS';}],
 ['boolean helper count','result',v=>{v.helperModesExecuted=false;}],
 ['extra helper mode','result',v=>{v.helperModesExecuted=1;}],
 ['claimed complete menu','result',v=>{v.originals.completeMainmenuVerified=true;}],
 ['string false menu claim','result',v=>{v.originals.completeMainmenuVerified='false';}],
 ['boolean original count','result',v=>{v.originals.count=true;}],
 ['boolean launcher width','result',v=>{v.originals.launcherNativeWidth=true;}],
 ['string game width','result',v=>{v.originals.gameNativeWidth='2';}],
 ['different final original hash','result',v=>{v.originals.capture003PngSha256='0'.repeat(64);}],
 ['promoted freshness','result',v=>{v.originals.freshness='FRESH';}],
 ['extra summary field','result',v=>{v.originals.extra=true;}],
 ['seed created','result',v=>{v.seed.created=true;}],
 ['string seed flag','result',v=>{v.seed.created='false';}],
 ['stability claim','result',v=>{v.seed.stabilityModeExecuted=true;}],
 ['successful seed closeout','result',v=>{v.seed.modernCloseoutCreated=true;}],
 ['legacy handoff observation','observation',v=>{v.kind='ACTUAL_DRIVER_HANDOFF_CUSTODY_STOP';}],
 ['changed capture category','observation',v=>{v.category='CAPTURE_UNCERTAIN';}],
 ['legacy guard projection','observation',v=>{v.guardHistory=v.actualGuardHistory;delete v.actualGuardHistory;}],
 ['boolean recapture count','observation',v=>{v.permittedLoadingRecapturesUsed=true;}],
 ['extra loading recapture','observation',v=>{v.permittedLoadingRecapturesUsed=2;}],
 ['boolean observation helper count','observation',v=>{v.helperModesExecuted=false;}],
 ['uninspected original','observation',v=>{v.originals[2].originalInspected=false;}],
 ['nonstring assertion','observation',v=>{v.originals[2].assertion=true;}],
 ['unbounded assertion','observation',v=>{v.originals[2].assertion='x'.repeat(257);}],
 ['extra original field','observation',v=>{v.originals[2].verified=true;}],
 ['seed claim','observation',v=>{v.seedClaim=true;}],
 ['unexpected observation field','observation',v=>{v.extra=true;}]
];
for(const [name,key,change]of cases)test('capture-stop refuses '+name+' in both real readers',()=>{
 const f=captureStopHistoryFixture();
 try{editCaptureEvidence(f,key,change);f.refresh();assertCaptureRefused(f);}finally{f.close();}
});
