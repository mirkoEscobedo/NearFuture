// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {seedCreationActionLimit} from '../seed-creation-history.mjs';
import {preContinueHistoryFixture} from './seed-pre-continue-history-test-support.mjs';

/** @param {any} options @param {boolean} expected */
function paired(options,expected){
    const f=preContinueHistoryFixture(options);
    try{
        let jsAccepted=false;try{jsAccepted=seedCreationActionLimit(f.plan,f.launchLease,f.scenario)===103;}catch{}
        const ps=f.admit();assert.equal(ps.error,undefined);assert.equal(ps.signal,null);
        assert.deepEqual({jsAccepted,psStatus:ps.status},{jsAccepted:expected,psStatus:expected?0:1},ps.stderr);
        if(!expected)assert.match(ps.stderr,/pre-Continue|closed|custody|history/i);
    }finally{f.close();}
}

test('menu inspection after E1 start is not a prior producer observation',()=>paired({menuInspectionOffsetMs:2500},false));
test('Generate inspection after its known send is not a BEFORE observation',()=>paired({generateInspectionOffsetMs:1000},false));
test('a different valid mapped B1 menu point cannot produce the archived menu coordinate',()=>paired({differentMappedMenuPoint:true},false));
test('a game born between controller creation and publication was never admitted',()=>paired({gameBirthOffsetMs:150},false));
test('a game born at controller publication retains the exact closed history',()=>paired({gameBirthOffsetMs:200},true));
