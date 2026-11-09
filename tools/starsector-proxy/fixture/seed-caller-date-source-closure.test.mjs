// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {dirname,resolve,relative,basename} from 'node:path';
import {ownedHttpSources} from '../owned-http-batch.mjs';
import {campaignLaunchSources} from '../campaign-authority.mjs';
import {fixtureProbeSources} from '../fixture-binding.mjs';
import {repository} from './seed-test-support.mjs';

test('new caller-date readers retain their deterministic transitive JS and native source inventory',()=>{
 /** @type {Set<string>} */const found=new Set();
 /** @param {string} path */function visit(path){
  if(found.has(path))return;found.add(path);
  const text=readFileSync(path,'utf8');
  for(const match of text.matchAll(/from\s+['"]([^'"]+)['"]/g))if(match[1].startsWith('.'))visit(resolve(dirname(path),match[1]));
 }
 visit(resolve(repository,'tools/starsector-proxy/seed-caller-date-attempt.mjs'));
 const native=['seed-caller-date-attempt.ps1','seed-caller-date-geometry.ps1','seed-zero-provider-terminal.ps1','read-owned-client-geometry.ps1'];
 const psInventories=['seed-fixture.ps1','seed-provenance-v2.ps1'].map(name=>readFileSync(resolve(repository,'tools/starsector-proxy/fixture',name),'utf8'));
 for(const path of found){
  const key=relative(repository,path).replaceAll('\\','/');
  for(const inventory of [ownedHttpSources,campaignLaunchSources])assert.ok(inventory.includes(key),key);
  for(const text of psInventories)assert.ok(text.includes("'"+basename(path)+"'"),basename(path));
 }
 for(const name of native){
  const key='tools/starsector-proxy/fixture/'+name;
  for(const inventory of [fixtureProbeSources,ownedHttpSources,campaignLaunchSources])assert.ok(inventory.includes(key),key);
  for(const text of psInventories)assert.ok(text.includes("'"+name+"'"),name);
 }
});
