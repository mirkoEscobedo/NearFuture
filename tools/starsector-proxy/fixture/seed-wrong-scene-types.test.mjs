// @ts-check
import test from 'node:test';
import {wrongSceneHistoryFixture} from './seed-wrong-scene-test-support.mjs';
import {assertCaptureRefused,editCaptureEvidence as editHistory} from './seed-capture-stop-test-support.mjs';

/** @type {[string,string,(v:any)=>void][]} */
const cases=[
 ['ready schema string','e1ProviderReady',v=>{v.schemaVersion='1';}],
 ['startup deadline string','e1ProviderReady',v=>{v.startupTimeoutMs='20000';}],
 ['parent PID string','e1ProviderReady',v=>{v.node.parentPid='204';}],
 ['node session string','e1ProviderReady',v=>{v.node.sessionId='1';}],
 ['helper session string','e1ProviderReady',v=>{v.helper.sessionId='1';}],
 ['batch array','e1ProviderReady',v=>{v.batch=['E1'];}],
 ['consent array','e1ProviderReady',v=>{v.consentMode=['finite-explicit-consent'];}],
 ['query array','e1ProviderReady',v=>{v.query=['QueryInformationJobObject(NULL,JobObjectBasicProcessIdList)'];}],
 ['native width string','result',v=>{v.originals.nativeGameWidth='2';}],
 ['cleanup flag string','e1ProviderTerminal',v=>{v.cleanupUnknown='false';}],
 ['terminal status string','e1ProviderTerminal',v=>{v.verifierStatus='0';}],
 ['returned step timeout string','e1ProviderTerminal',v=>{v.steps[0].timedOut='false';}],
];
for(const [name,key,change]of cases)test('both history readers reject '+name,()=>{
 const f=wrongSceneHistoryFixture();
 try{editHistory(f,key,change);f.refresh();assertCaptureRefused(f);}finally{f.close();}
});
