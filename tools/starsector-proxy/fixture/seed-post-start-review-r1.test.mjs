// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {postStartHistoryFixture} from './seed-post-start-history-test-support.mjs';
import {seedCreationActionLimit} from '../seed-creation-history.mjs';

/** @param {any} options @param {boolean} expected */
function paired(options,expected){
    const f=postStartHistoryFixture(options);
    try{
        let jsAccepted=false;try{jsAccepted=seedCreationActionLimit(f.plan,f.launchLease,f.scenario)===99;}catch{}
        const ps=f.admit();assert.equal(ps.error,undefined);assert.equal(ps.signal,null);
        assert.deepEqual({jsAccepted,psStatus:ps.status},{jsAccepted:expected,psStatus:expected?0:1},ps.stderr);
        if(!expected)assert.match(ps.stderr,/post-Start|Shared post-Start|previous seed|Campaign authority/i);
    }finally{f.close();}
}
for(const [name,options] of /** @type {[string,any][]} */([
    ['B1 fresh pre-Play window moved',{movedWindow:{batch:'B1',call:6}}],
    ['E2 fresh pre-Start window moved',{movedWindow:{batch:'E2',call:24}}],
    ['B1 closed mapping precedes known closure',{geometryReview:{timingLabel:'B1-NEW_GAME',inspectionOffsetMs:-2000}}],
    ['B1 closed mapping follows E1 startup',{geometryReview:{timingLabel:'B1-NEW_GAME',inspectionOffsetMs:14000}}],
    ['owned snapshot follows its geometry start',{geometryReview:{snapshotLabel:'E2-START_GAME',snapshotOffsetMs:50}}],
    ['HWND singleton arrays',{geometryReview:{handle:(/** @type {number} */ n)=>[n]}}],
    ['HWND whitespace text',{geometryReview:{handle:(/** @type {number} */ n)=>' '+n}}],
    ['HWND decimal text',{geometryReview:{handle:(/** @type {number} */ n)=>n+'.0'}}],
    ['HWND exponent text',{geometryReview:{handle:(/** @type {number} */ n)=>n+'e0'}}],
]))test(name+' is rejected through both public histories',()=>paired(options,false));

test('numeric HWNDs remain admitted',()=>paired({},true));
test('inspection may precede B1 closure while its geometry follows closure',()=>paired({geometryReview:{
    timingLabel:'B1-NEW_GAME',inspectionOffsetMs:-2000,geometryDelayMs:2000,snapshotLabel:'B1-NEW_GAME',snapshotOffsetMs:5,
}},true));
test('canonical decimal HWNDs and snapshots after inspection but before geometry remain admitted',()=>paired({
    geometryReview:{handle:(/** @type {number} */ n)=>String(n),snapshotLabel:'E2-START_GAME',snapshotOffsetMs:5},
},true));
