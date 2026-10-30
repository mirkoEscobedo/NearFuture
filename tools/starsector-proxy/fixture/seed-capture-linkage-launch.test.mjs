// @ts-check
import test from 'node:test';
import {captureLinkageHistoryFixture} from './seed-capture-linkage-test-support.mjs';
import {value} from './seed-failed-history-test-support.mjs';
import {json} from './seed-test-support.mjs';
import {assertCaptureRefused} from './seed-capture-stop-test-support.mjs';
for(const [label,change]of /** @type {[string,(v:any)=>void][]} */([
 ['command',v=>{v.command='unrelated.exe';}],['arguments',v=>{v.args=['unrelated'];}],['working directory',v=>{v.cwd='unrelated';}]
])){
 test('both readers bind original launch '+label,()=>{
  const f=captureLinkageHistoryFixture();try{const p=f.evidencePaths.launch,v=value(p);change(v.result.steps[0]);json(p,v);f.refresh();assertCaptureRefused(f);}finally{f.close();}
 });
}
test('both readers refuse a coherently propagated different root command',()=>{
 const f=captureLinkageHistoryFixture({changeRootCommand:v=>v+' -Unexpected'});try{assertCaptureRefused(f);}finally{f.close();}
});
