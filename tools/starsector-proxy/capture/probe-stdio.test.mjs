// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import { writeFileSync,readFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { resolve } from 'node:path';
import { makeOutput } from './test-output.mjs';
test('observation server stderr overflow still records actual owned cleanup and refuses further calls', {timeout:15000}, () => {
 const output=makeOutput('stdio-overflow-');
 const fixture=resolve(output,'overflow-server.mjs');
 writeFileSync(fixture,"process.stderr.write('x'.repeat(70000));process.stdin.resume();setInterval(()=>{},1000);\n");
 const actualPath=resolve(output,'actual.private.json');
 const input={node:process.execPath,entrypoint:fixture,configuration:{path:'UNUSED_SYNTHETIC',sha256:'0'.repeat(64)},workspaceRoot:output,environment:{},actualPath};
 const bytes=JSON.stringify(input),inputPath=resolve(output,'input.private.json');writeFileSync(inputPath,bytes);
 const cli=fileURLToPath(new URL('probe-stdio.mjs',import.meta.url));
 const result=spawnSync(process.execPath,[cli,inputPath,createHash('sha256').update(bytes).digest('hex')],{encoding:'utf8',timeout:10000,windowsHide:true});
 assert.equal(result.error,undefined,'owned diagnostic must close within its own await flow');
 assert.notEqual(result.status,0);
 const actual=JSON.parse(readFileSync(actualPath,'utf8'));
 assert.equal(actual.serverStderrOverflow,true);
 assert.equal(actual.status,'HOLD');
 assert.equal(actual.closeReturned,true);
 assert.ok(actual.serverPid>0);
 assert.equal(actual.serverPidAbsent,true);
 assert.deepEqual(actual.calls,[]);
 assert.ok(actual.serverStderr.length<=65536);
});
