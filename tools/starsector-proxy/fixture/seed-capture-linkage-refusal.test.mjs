// @ts-check
import test from 'node:test';
import {captureLinkageHistoryFixture} from './seed-capture-linkage-test-support.mjs';
import {value,ref} from './seed-failed-history-test-support.mjs';
import {json} from './seed-test-support.mjs';
import {assertCaptureRefused} from './seed-capture-stop-test-support.mjs';

test('both public readers refuse a present nonboolean raw error flag after coherent repinning',()=>{
 const f=captureLinkageHistoryFixture();
 try{
  const path=f.calls[0].raw.path,raw=value(path);raw.isError='false';json(path,raw);
  const o=value(f.evidencePaths.observation);o.B1.calls[0].raw=ref(path);json(f.evidencePaths.observation,o);
  f.refresh();assertCaptureRefused(f);
 }finally{f.close();}
});

test('both public readers refuse a nonpositive initial owned Job PID',()=>{
 const f=captureLinkageHistoryFixture({initialJobPids:[100,105,101,0]});
 try{assertCaptureRefused(f);}finally{f.close();}
});
