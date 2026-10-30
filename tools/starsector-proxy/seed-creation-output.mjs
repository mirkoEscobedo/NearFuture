// @ts-check
import {resolve,dirname} from 'node:path';
import {lstatSync,readdirSync} from 'node:fs';
import {isDeepStrictEqual} from 'node:util';
import {isInitialFocusCreation} from './seed-creation-profile.mjs';
import {requireThat,file} from './campaign-proof.mjs';

/** Only fixed preference output after Continue, then one initial-save operation.
 * Partial save files may be observed while the known Start runs; they never prove a seed.
 * @param {any} binding @param {any} state @param {boolean} input */
export function assertSeedCreationOutput(binding,state,input) {
    const saves=resolve(binding.scenario,'installation/saves');
    for(let p=saves;;p=dirname(p)) {
        requireThat(!lstatSync(p).isSymbolicLink(),'ordinary seed creation saves required');
        if(dirname(p)===p)break;
    }
    requireThat(lstatSync(saves).isDirectory(),'seed creation save directory required');
    const entries=readdirSync(saves);
    if(entries.length===0)return;
    const continued=state.pending?.batch==='E1'&&state.pending.wizardStep?.stage==='CONTINUE'&&
        state.pending.status==='AWAITING_AFTER'||state.resolvedE1?.wizardStep?.stage==='CONTINUE'&&
        state.resolvedE1.status==='AWAITING_AFTER';
    requireThat((binding.schemaVersion===6||isInitialFocusCreation(binding))&&state.uncertain===false&&continued,'known seed Continue required before output');
    const common=resolve(saves,'common'),settings=resolve(common,'core_shared_settings.json.data');
    requireThat(entries.includes('common')&&lstatSync(common).isDirectory()&&!lstatSync(common).isSymbolicLink()&&
        isDeepStrictEqual(readdirSync(common),['core_shared_settings.json.data'])&&lstatSync(settings).nlink===1&&
        file(settings,41).bytes.equals(Buffer.from('{"campaignHelpPopupsOptionChecked": true}')),'fixed seed common preference required');
    const saved=entries.filter(name=>name!=='common');
    if(saved.length===0)return;
    requireThat(!input&&binding.batch==='E2'&&state.pending?.wizardStep?.stage==='START_GAME'&&
        state.pending.status==='AWAITING_AFTER'&&saved.length===1&&/^save_[A-Za-z0-9_]{1,128}$/.test(saved[0]),
    'only terminal seed initial-save observations permitted');
    const dir=resolve(saves,saved[0]),stat=lstatSync(dir);
    requireThat(stat.isDirectory()&&!stat.isSymbolicLink(),'ordinary generated seed directory required');
    let total=0;
    for(const name of readdirSync(dir)) {
        requireThat(['campaign.xml','descriptor.xml'].includes(name),'unrelated seed output refused');
        const path=resolve(dir,name),s=lstatSync(path);
        requireThat(s.isFile()&&!s.isSymbolicLink()&&s.nlink===1&&s.size<=16*1024*1024,'bounded ordinary seed output required');
        total+=s.size;
    }
    requireThat(total<=16*1024*1024,'bounded generated seed output required');
}
