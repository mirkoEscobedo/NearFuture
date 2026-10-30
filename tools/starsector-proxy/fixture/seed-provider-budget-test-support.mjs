// @ts-check
import {initialFocusFixture} from './seed-initial-focus-test-support.mjs';
import {requiredOwnedHttpFiles} from '../owned-http-batch.mjs';
import {directFixture} from '../direct-test-fixture.mjs';
import {seedNativeSessionFactory} from './campaign-input-test-support.mjs';
import {json,filePin} from './seed-test-support.mjs';
import {resolve} from 'node:path';
import {readFileSync,realpathSync} from 'node:fs';

/** Generated opt-in authority; no provider or native process is started here. */
export function providerBudgetFixture(){
    const f=initialFocusFixture();
    const timing={schemaVersion:1,profile:'INITIAL_FOCUS_PROVIDER_480000',providerTimeoutMs:480000};
    for(const value of [f.plan,f.launchLease,f.phasePlan,f.phaseLease])value.providerTiming=timing;
    f.plan.bounds.proxyTimeoutMs=f.phasePlan.bounds.proxyTimeoutMs=480000;
    f.refreshLaunch();
    return f;
}

/** Real provider admission/HTTP/consent/claims, generated native and spawn edges.
 * @param {boolean} [selected] */
export function providerBudgetHost(selected=true){
    const envNames=['COMPUTER_USE_REQUIRE_APPROVAL','COMPUTER_USE_APPROVAL_TOKEN','COMPUTER_USE_AUDIT_LOG'];
    const previous=envNames.map(name=>process.env[name]);
    process.env.COMPUTER_USE_REQUIRE_APPROVAL='true';delete process.env.COMPUTER_USE_APPROVAL_TOKEN;
    process.env.COMPUTER_USE_AUDIT_LOG='false';
    const f=selected?providerBudgetFixture():initialFocusFixture();
    const powershell=realpathSync.native(process.env.NF_TEST_POWERSHELL??'pwsh');
    const supervisor=resolve(f.root,'node_modules/workspace-template-win32-x64/bin/workspace-template.exe');
    const manifest=f.launchLease.supportPinsPath;
    json(manifest,{pins:requiredOwnedHttpFiles(f.root,powershell,supervisor).map(path=>({path,sha256:filePin(path).sha256.toLowerCase()}))});
    f.launchLease.supportPinsSha256=f.phaseLease.supportPinsSha256=filePin(manifest).sha256.toLowerCase();
    f.refreshLaunch();
    const authority=f.launcher('provider-budget-'+resolve(f.scenario).split(/[\\/]/).at(-1));
    const provider=directFixture({authority:{root:f.root,source:authority.source,runID:authority.runID,powershell,supervisor,consentMode:'finite-explicit-consent'},
        sessionFactory:seedNativeSessionFactory(()=>{})});
    return {f,provider,ledger:()=>JSON.parse(readFileSync(resolve(f.scenario,'guard-launch-history.private.json'),'utf8')),
        async close(){try{await provider.close();f.close();}finally{
            envNames.forEach((name,i)=>{if(previous[i]===undefined)delete process.env[name];else process.env[name]=previous[i];});
        }}};
}
