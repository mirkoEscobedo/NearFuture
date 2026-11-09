// @ts-check
import test from 'node:test';
import {wrongSceneHistoryFixture} from './seed-wrong-scene-test-support.mjs';
import {assertCaptureRefused,editCaptureEvidence as editHistory} from './seed-capture-stop-test-support.mjs';
import {value} from './seed-failed-history-test-support.mjs';
import {json} from './seed-test-support.mjs';

/** @type {[string,Parameters<typeof wrongSceneHistoryFixture>[0]][]} */
const bound=[
 ['target app array',{changeE1Args:v=>{v.target_app=['java.exe'];}}],
 ['focus strategy array',{changeE1Args:v=>{v.focus_strategy=['strict'];}}],
 ['target HWND string',{changeE1Args:v=>{v.target_window_id='20';}}],
 ['foreground PID string',{changeE1Front:v=>{v.structuredContent.app.pid='101';}}],
 ['foreground app array',{changeE1Front:v=>{v.structuredContent.app.bundleId=['java.exe'];}}],
];
for(const [name,options]of bound)test('both readers refuse coherent '+name,()=>{
 const f=wrongSceneHistoryFixture(options);try{assertCaptureRefused(f);}finally{f.close();}
});
/** @type {[string,string,(v:any)=>void][]} */
const direct=[
 ['observation schema string','observation',v=>{v.schemaVersion='1';}],
 ['owned PID string','observation',v=>{v.ownedPid='101';}],
 ['launcher HWND string','observation',v=>{v.launcherWindow='19';}],
 ['game HWND string','observation',v=>{v.gameWindow='20';}],
 ['provider batch array','observation',v=>{v.providers[1].batch=['E1'];}],
 ['original freshness array','observation',v=>{v.providers[1].originals[0].freshness=['UNKNOWN'];}],
 ['summary freshness array','result',v=>{v.originals.freshness=['UNKNOWN'];}],
];
for(const [name,key,change]of direct)test('both readers refuse '+name,()=>{
 const f=wrongSceneHistoryFixture();try{editHistory(f,key,change);f.refresh();assertCaptureRefused(f);}finally{f.close();}
});
test('both readers refuse coherently pinned array tool names in original call and attempt',()=>{
 const f=wrongSceneHistoryFixture();try{
  const calls=value(f.evidencePaths.observation).providers[1].calls;
  for(const k of ['post','attempt']){const p=calls[2][k].path,v=value(p);v.tool=['get_window'];json(p,v);}
  f.refresh();assertCaptureRefused(f);
 }finally{f.close();}
});
