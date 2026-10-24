// @ts-check
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {mkdtempSync,mkdirSync,writeFileSync,readFileSync,unlinkSync,rmSync} from 'node:fs';
import {resolve,relative,isAbsolute} from 'node:path';
import {fileURLToPath} from 'node:url';
import {spawnSync} from 'node:child_process';
import {seedFixture,json,write,powershell} from './seed-test-support.mjs';

export const repository=resolve(fileURLToPath(new URL('../../../',import.meta.url)));
const scenarios=resolve(repository,'.tmp/starsector-proxy/scenarios');
const quote=(/** @type {string} */ value)=>"'" + value.replaceAll("'","''") + "'";
const digest=(/** @type {string} */ value)=>createHash('sha256').update(value).digest('hex');

export function generationBindingFixture() {
    mkdirSync(scenarios,{recursive:true});
    const root=mkdtempSync(resolve(scenarios,'generation-binding-test-'));
    for (const name of ['source','runtime']) mkdirSync(resolve(root,name));
    const agent=resolve(root,'test-agent.jar');
    writeFileSync(agent,'generated boundary agent');
    const binding={schemaVersion:3,fixtureRoot:root,sourceInstallation:resolve(root,'source'),
        qualifiedRuntimeRoot:resolve(root,'runtime'),candidateHead:'a'.repeat(40),
        runtimeManifestSha256:'b'.repeat(64),presentationAgentPath:agent,
        presentationAgentSha256:digest('generated boundary agent'),presentationClassSha256:'c'.repeat(64),
        driverRole:'/root/recovery_game_driver',fixtureProfile:'VANILLA_UNCOMPRESSED_GENERATION'};
    const path=resolve(root,'fixture-binding.private.json');
    const text=JSON.stringify(binding);writeFileSync(path,text);
    return {root,path,binding,sha256:digest(text),close() {
        const suffix=relative(scenarios,root);
        assert.ok(suffix&&!suffix.startsWith('..')&&!isAbsolute(suffix),'owned generated boundary cleanup only');
        rmSync(root,{recursive:true});
    }};
}

/** Real public PowerShell binding seam; no fixture preparation or game.
 * @param {ReturnType<typeof generationBindingFixture>} fixture
 * @param {string} [afterRead] */
export function readGenerationBinding(fixture,afterRead='$binding | ConvertTo-Json -Depth 6') {
    const powershell=process.env.NF_TEST_POWERSHELL ?? '';
    assert.ok(isAbsolute(powershell),'explicit absolute NF_TEST_POWERSHELL required');
    const source=resolve(repository,'tools/starsector-proxy/fixture/fixture-inputs.ps1');
    return spawnSync(powershell,['-NoProfile','-NonInteractive','-Command',
        "$ErrorActionPreference='Stop'; . "+quote(source)+"; $binding=Read-NativeFixtureBinding "+
        quote(fixture.path)+" "+quote(fixture.sha256)+"; "+afterRead],
        {cwd:repository,encoding:'utf8',timeout:15000});
}

/** Generated boundary data only; no game or native provenance. */
export function generationProtectedFixture() {
    const f=seedFixture();
    const binding=JSON.parse(readFileSync(f.bindingPath,'utf8'));
    binding.schemaVersion=3;binding.fixtureProfile='VANILLA_UNCOMPRESSED_GENERATION';
    const bindingPin=json(f.bindingPath,binding);
    for(const name of ['seed-manifest.private.json','seed-stage-plan.private.json','seed-stage-authority.private.json']) {
        unlinkSync(resolve(f.target,name));
    }
    const settingsPath=resolve(f.installation,'starsector-core/data/config/settings.json');
    const modsPath=resolve(f.installation,'mods/enabled_mods.json');
    const receipt=JSON.parse(readFileSync(f.frozenPath,'utf8'));
    function configure(/** @type {boolean} */ compressed,/** @type {string[]} */ mods=[]) {
        receipt.settingsSha256=write(settingsPath,JSON.stringify({autosaveFeatureEnabled:false,
            autosaveEnabledByDefault:false,compressSaveGameData:compressed})).sha256;
        receipt.enabledModsSha256=json(modsPath,{enabledMods:mods}).sha256;
        json(f.frozenPath,receipt);
    }
    configure(false);
    return {...f,binding,bindingPin,configure,protection(phase='after-smoke') {
        return powershell(['-File',resolve(repository,'tools/starsector-proxy/fixture/protection-check.ps1'),
            '-BindingPath',f.bindingPath,'-BindingSha256',bindingPin.sha256,'-Phase',phase,
            '-EvidenceRoot',f.admission]);
    }};
}
