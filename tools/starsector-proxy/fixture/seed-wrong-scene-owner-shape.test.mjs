// @ts-check
import test from 'node:test';
import {wrongSceneHistoryFixture} from './seed-wrong-scene-test-support.mjs';
import {assertCaptureRefused} from './seed-capture-stop-test-support.mjs';

/** @type {[string,Parameters<typeof wrongSceneHistoryFixture>[0]][]} */
const cases=[
 ['missing original kernel birth',{changeB1Owned:v=>{delete v.processes[0].kernelCreationFileTime;}}],
 ['missing original parent',{changeB1Owned:v=>{delete v.processes[0].parentPid;}}],
 ['wrong original launcher handle',{changeB1Owned:v=>{v.processes[0].mainWindowHandle=99;}}],
 ['wrong E1 owned handle',{changeE1Owned:v=>{v.processes[0].mainWindowHandle=99;}}],
 ['session string',{sessionId:'1'}],
 ['noninteractive session zero',{sessionId:0}],
];
for(const [name,options]of cases)test('both readers require full original owner: '+name,()=>{
 const f=wrongSceneHistoryFixture(options);
 try{assertCaptureRefused(f);}finally{f.close();}
});
