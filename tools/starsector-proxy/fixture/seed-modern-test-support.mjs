// @ts-check
// Composed real transport with generated OS/visual/cleanup boundaries. Never native evidence.
import assert from 'node:assert/strict';
import {resolve,basename} from 'node:path';
import {readFileSync} from 'node:fs';
import {exerciseSeedCreationTransport} from './seed-creation-transport-support.mjs';
import {seedFixture,json,filePin,powershell as invokePowershell} from './seed-test-support.mjs';
import {requiredOwnedHttpFiles} from '../owned-http-batch.mjs';
/** @param {string} p */
const reference=p=>({path:p,sha256:filePin(p).sha256.toLowerCase()});
export async function modernSeedFixture() {
    const transport=await exerciseSeedCreationTransport({retain:true});assert.ok(transport);
    const source=transport.fixture,target=seedFixture({driverRole:'/root/recovery_game_driver'}),saveDirectory='save_created_fixture';
    const rootCallPath=resolve(source.scenario,'root-call.actual.private.json');json(rootCallPath,source.campaign.rootCallRecord);
    const afterPath=resolve(source.scenario,'after-smoke-protection.private.json');
    const preference=JSON.parse(readFileSync(source.preferencePath,'utf8'));
    const protection={phase:'after-smoke',protectedOriginalFiles:182,protectedSaveFiles:178,allOriginalAndBackupHashesMatch:true,
        saveCensusUnchanged:true,liveGameProcesses:0,originalWrites:false,privateAutosaveEnabled:false,privateSaveEntries:2,
        originalAutosave:{enabled:true,maximumSlots:3},sharedPreference:preference};json(afterPath,protection);
    const launchPath=resolve(source.scenario,'launch-supervisor.private.json');
    json(launchPath,{schemaVersion:1,command:'verify',ok:true,result:{verdict:'FAIL',steps:[{status:1,timedOut:false,processOwnership:'windows-job-object'}]}});
    const ledgerPath=resolve(source.scenario,'guard-launch-history.private.json'),ledger=JSON.parse(readFileSync(ledgerPath,'utf8'));
    const evidence={fixtureReceipt:reference(source.frozenPath),rootCall:reference(rootCallPath),protection:reference(afterPath),
        launch:reference(launchPath),plan:reference(source.planPath),lease:reference(source.leasePath),controller:reference(source.controllerPath),
        ledger:reference(ledgerPath),preference:reference(source.preferencePath),historyAdmission:reference(source.grantPath)};
    const files=['campaign.xml','descriptor.xml'].map(relative=>({relative,...filePin(resolve(source.installation,'saves',saveDirectory,relative))}));
    const closeoutPath=resolve(source.scenario,'seed-creation-closeout.private.json');
    const closeout={schemaVersion:1,kind:'ACTUAL_DISPOSABLE_SEED_CREATION_CLOSEOUT',scenarioId:basename(source.scenario),
        toolingHead:source.plan.source.qualifiedHead,rootSessionId:source.campaign.rootCallRecord.value.raw.session_id,allOwnedHandlesTerminalReaped:true,
        actions:{inheritedBaseline:source.campaign.historyBaseline,actualGuard:ledger.history,currentRunContext:{overallAttempts:41,uiAttempts:38},
            absoluteMaxReservedActions:80,clicks:8,activations:0,providers:3},
        cleanup:{verdict:'FAIL',status:1,timedOut:false,processOwnership:'windows-job-object',zeroLiveGameProcesses:true},
        protection,files,evidence};json(closeoutPath,closeout);
    const manifest={schemaVersion:2,kind:'NATIVE99_DISPOSABLE_SEED_MANIFEST_V2',sourceWorkspace:source.root,
        sourceScenario:basename(source.scenario),sourceEvidenceScenario:basename(source.scenario),sourceRun:41,
        saveDirectory,gameVersion:'0.98a-RC8',files,
        provenance:{fixtureReceipt:filePin(source.frozenPath),rootCall:filePin(rootCallPath),closeout:filePin(closeoutPath),
            protection:filePin(afterPath),launch:filePin(launchPath)}};
    const discovery=invokePowershell(['-Command','(Get-Process -Id $PID).Path']);
    assert.equal(discovery.status,0,discovery.stderr);
    const powershell=discovery.stdout.trim();assert.ok(powershell);
    const supervisor=resolve(source.root,'node_modules/workspace-template-win32-x64/bin/workspace-template.exe');
    const extras=['prepare-fixture.ps1','run-native.ps1','read-owned-client-geometry.ps1'].map(n=>resolve(source.root,'tools/starsector-proxy/fixture',n));
    const pins=[...new Set([...requiredOwnedHttpFiles(source.root,powershell,supervisor),...extras])].map(p=>({path:p,sha256:filePin(p).sha256}));
    const authority=JSON.parse(readFileSync(target.authorityPath,'utf8'));
    const planPath=resolve(target.target,'seed-stage-plan.private.json'),plan=JSON.parse(readFileSync(planPath,'utf8'));
    authority.sourcePinsSha256=json(resolve(target.target,'source-pins.private.json'),{pins}).sha256;
    function refresh() {
        authority.manifestSha256=json(target.manifestPath,manifest).sha256;
        plan.seedStage.manifestSha256=authority.manifestSha256;
        authority.planSha256=json(planPath,plan).sha256;
        json(target.authorityPath,authority);
    }
    refresh();
    return {source,target,manifest,closeout,closeoutPath,ledgerPath,
        commonPath:resolve(source.installation,'saves/common/core_shared_settings.json.data'),refresh,
        close(){target.close();transport.close();}};
}
