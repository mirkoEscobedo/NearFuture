// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {clientImagePoint} from './client-coordinate.mjs';

const window={windowId:19,pid:101,isFocused:true,bounds:{x:100,y:200,width:800,height:600}};
const geometry={schemaVersion:1,scope:'READ_ONLY_OWNED_HWND_CLIENT_GEOMETRY',windowId:19,pid:101,
 creationTime:'2026-10-26T06:00:00.000Z',controllerPid:100,observedAtUtc:'2026-10-26T06:00:01.000Z',
 outerBounds:window.bounds,clientOriginScreen:{x:110,y:240},clientSize:{width:780,height:550},
 offsetFromOuter:{x:10,y:40},mcpOuterBoundsMatched:true,inputGranted:false,originMeaning:'Generated client origin'};
const owned={pid:101,creationTime:geometry.creationTime,controllerPid:100,inSupervisorJob:true};
const input={window,geometry,owned,dimensions:{width:800,height:600},point:{x:50,y:60},
 nowMs:Date.parse('2026-10-26T06:00:02.000Z')};

test('native-width client image point uses measured client origin rather than outer origin',()=>{
 assert.deepEqual(clientImagePoint(input),[160,300]);
 assert.notDeepEqual(clientImagePoint(input),[150,260]);
});

/** @type {[string,(v:any)=>void][]} */
const refusals=[
 ['outer origin substituted',v=>{v.geometry.clientOriginScreen={x:100,y:200};}],
 ['unmeasured origin',v=>{delete v.geometry;}],
 ['different HWND',v=>{v.geometry.windowId=20;}],
 ['different process',v=>{v.geometry.pid=102;}],
 ['different process birth',v=>{v.geometry.creationTime='2026-10-26T05:59:00.000Z';}],
 ['different controller',v=>{v.geometry.controllerPid=200;}],
 ['unowned process',v=>{v.owned.inSupervisorJob=false;}],
 ['lost focus',v=>{v.window.isFocused=false;}],
 ['moved outer window',v=>{v.window.bounds.x=101;}],
 ['changed client origin',v=>{v.geometry.offsetFromOuter.x=11;}],
 ['resized image',v=>{v.dimensions.width=400;}],
 ['right bitmap padding',v=>{v.point.x=790;}],
 ['bottom bitmap padding',v=>{v.point.y=560;}],
 ['negative local point',v=>{v.point.x=-1;}],
 ['fractional local point',v=>{v.point.y=60.5;}],
 ['nonfinite origin',v=>{v.geometry.clientOriginScreen.x=Infinity;}],
 ['stale geometry',v=>{v.nowMs+=5001;}],
 ['future geometry',v=>{v.nowMs-=2000;}],
 ['malformed geometry time',v=>{v.geometry.observedAtUtc='bad';}],
 ['unmatched MCP bounds',v=>{v.geometry.mcpOuterBoundsMatched=false;}],
 ['geometry cannot grant input',v=>{v.geometry.inputGranted=true;}],
];
for(const [name,change] of refusals)test('rejects '+name,()=>{
 const v=structuredClone(input);change(v);assert.throws(()=>clientImagePoint(v),/client.*coordinate/i);
});
