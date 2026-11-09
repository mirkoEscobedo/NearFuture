// @ts-check
import test from 'node:test';
import {wrongSceneHistoryFixture} from './seed-wrong-scene-test-support.mjs';
import {assertCaptureRefused} from './seed-capture-stop-test-support.mjs';

/** @type {[string,(v:any)=>void][]} */
const cases=[
 ['parent',v=>{v.processes[0].parentPid++;}],
 ['birth',v=>{v.processes[0].creationTime='2026-10-25T00:00:00.000Z';}],
 ['kernel birth',v=>{v.processes[0].kernelCreationFileTime+='1';}],
 ['session',v=>{v.processes[0].sessionId++;}],
 ['executable',v=>{v.processes[0].executable+='-other';}],
 ['supervisor ownership',v=>{v.processes[0].inSupervisorJob=false;}],
 ['controller Job member',v=>{v.processes[0].supervisorJobPids=[101];}],
];
for(const [name,changeE1Owned]of cases)test('coherently pinned E1 '+name+' cannot replace original Play process',()=>{
 const f=wrongSceneHistoryFixture({changeE1Owned});
 try{assertCaptureRefused(f);}finally{f.close();}
});
