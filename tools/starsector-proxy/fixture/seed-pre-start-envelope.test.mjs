// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {syntheticPng} from '../direct-test-fixture.mjs';
import {preStartOriginal} from '../seed-pre-start-margin-records.mjs';

test('historical original accepts the vendor image plus matching dimensions text without changing PNG bytes',()=>{
    const raw={content:[{type:'image',mimeType:'image/png',data:syntheticPng},{type:'text',text:'1x1'}]};
    const window={bounds:{x:0,y:0,width:1,height:1}};
    assert.deepEqual(preStartOriginal(raw,window),Buffer.from(syntheticPng,'base64'));
});

test('historical original accepts a bare vendor image without inventing metadata',()=>{
    const raw={content:[{type:'image',mimeType:'image/png',data:syntheticPng}]};
    assert.deepEqual(preStartOriginal(raw,{bounds:{x:7,y:8,width:1,height:1}}),Buffer.from(syntheticPng,'base64'));
});

/** @type {[string,(v:any)=>void][]} */
const changes=[
    ['wrong dimension caption',v=>{v.content[1].text='2x1';}],
    ['unexpected caption field',v=>{v.content[1].authority=true;}],
    ['error flag as string',v=>{v.isError='false';}],
    ['extra image block',v=>{v.content.push(v.content[0]);}],
    ['noncanonical base64',v=>{v.content[0].data+='\n';}],
    ['truncated PNG',v=>{v.content[0].data=Buffer.from(syntheticPng,'base64').subarray(0,40).toString('base64');}],
    ['non-image content',v=>{v.content[0].type='text';}],
];
for(const [label,change] of changes)test('historical screenshot refuses '+label,()=>{
    const raw={content:[{type:'image',mimeType:'image/png',data:syntheticPng},{type:'text',text:'1x1'}]};
    change(raw);assert.throws(()=>preStartOriginal(raw,{bounds:{x:0,y:0,width:1,height:1}}));
});
