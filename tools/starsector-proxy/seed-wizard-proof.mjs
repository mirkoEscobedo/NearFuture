// @ts-check
import {resolve} from 'node:path';
import {isDeepStrictEqual} from 'node:util';
import {requireThat,read,file,closed,knownProviderClosure} from './campaign-proof.mjs';
import {completePng} from './direct-frames.mjs';

const controls={
    E1:[['NEW_GAME','STARSECTOR_MAIN_MENU','New Game'],['GENERATE','IDENTITY_EMPTY','GENERATE'],
        ['CONTINUE','IDENTITY_NAMED','1. Continue']],
    E2:[['EXPLORER','OCCUPATION_SELECTION','An explorer, leading a salvage expedition from aboard an Apogee-class cruiser (faster start)'],
        ['NORMAL','CAMPAIGN_DIFFICULTY','Normal'],['SKIP','TUTORIAL_CHOICE','Skip it'],
        ['START_GAME','NEW_GAME_STATS','Start game']],
};

/** Original pixels, raw transport and current inspection are independently bound.
 * @param {any} binding @param {any} previous @param {any} frame @param {string} prefix
 * @param {number} now @param {any} [after] */
function original(binding,previous,frame,prefix,now,after) {
    const pngPath=resolve(binding.scenario,prefix,'F1.original.png');
    const metadataPath=resolve(binding.scenario,prefix,'capture-001.metadata.private.json');
    const rawPath=resolve(binding.scenario,prefix,'capture-001.raw.private.json');
    requireThat(closed(frame,['pngPath','pngSha256','metadataPath','metadataSha256','capturedAtMs'])&&
        frame.pngPath===pngPath&&frame.metadataPath===metadataPath,'fixed seed wizard original paths required');
    const png=file(pngPath,32*1024*1024),metadata=read(metadataPath),m=metadata.value;
    const raw=read(rawPath,32*1024*1024),images=raw.value.content?.filter((/** @type {any} */ v)=>v.type==='image');
    const dimensions=completePng(png.bytes),w=m.window;
    requireThat(frame.pngSha256===png.sha256&&frame.metadataSha256===metadata.sha256&&raw.value.isError!==true&&
        images?.length===1&&images[0].mimeType==='image/png'&&images[0].data===png.bytes.toString('base64')&&
        m.captureReceipt===true&&m.rawPath===rawPath&&m.rawSha256===raw.sha256&&m.pngPath===pngPath&&m.pngSha256===png.sha256&&
        m.runID===binding.runID&&m.batch===binding.batch&&m.sourceSha256===binding.sourceSha256&&
        m.reservationID===previous.reservationID&&m.capturedAtMs===frame.capturedAtMs&&
        m.args?.target_window_id===binding.campaign.windowId&&m.args.quality===0&&m.args.width===w?.bounds?.width&&
        w.windowId===binding.campaign.windowId&&binding.ownedPids.includes(w.pid)&&w.isFocused===true&&
        isDeepStrictEqual(m.dimensions,dimensions)&&dimensions.width===w.bounds.width&&dimensions.height===w.bounds.height&&
        Number.isSafeInteger(previous.knownResultAtMs)&&Number.isSafeInteger(frame.capturedAtMs)&&
        frame.capturedAtMs>=previous.knownResultAtMs&&frame.capturedAtMs<=now,'seed wizard original custody changed');
    if(after)requireThat(png.bytes.equals(after.pngBytes)&&isDeepStrictEqual(w,after.window)&&
        frame.capturedAtMs>=after.capturedAtMs&&frame.capturedAtMs<=after.observedAtMs,'current seed wizard original changed');
    return {window:w,png};
}

/** @param {any} value */
const validName=value=>typeof value==='string'&&value.trim().length>0&&value.length<=128&&!/[\u0000-\u001f\u007f]/u.test(value);

/** The only new input decisions: fixed labels and states, exact inspected coordinates.
 * @param {any} binding @param {any} state @param {any} args @param {any} after @param {number} now */
export function seedWizardInput(binding,state,args,after,now) {
    const completed=(state.resolvedActions??[]).filter((/** @type {any} */ a)=>a.batch===binding.batch&&a.runID===binding.runID&&a.sourceSha256===binding.sourceSha256);
    const ordinal=completed.length+(state.pending?2:1);
    if(binding.batch==='B1') {requireThat(!state.pending&&ordinal===1,'one seed launcher action required');return {ordinal,stage:'PLAY'};}
    const row=controls[/** @type {'E1'|'E2'} */ (binding.batch)]?.[ordinal-1];
    requireThat(row,'seed wizard sequence exhausted');
    if(binding.batch==='E1'&&ordinal===1) {
        requireThat(!state.pending&&isDeepStrictEqual(args.coordinate,binding.b1MenuReceipt.newGameCoordinate),'only inspected seed New Game required');
        return {ordinal,stage:row[0]};
    }
    const previous=state.pending??state.resolvedE1;
    requireThat(previous?.status==='AWAITING_AFTER'&&previous.tool==='left_click'&&
        (state.pending?previous.batch===binding.batch&&previous.runID===binding.runID&&previous.sourceSha256===binding.sourceSha256
            :binding.batch==='E2'&&ordinal===1&&previous.batch==='E1'), 'known preceding seed wizard action required');
    const prefix=binding.batch+'-wizard-'+ordinal,path=resolve(binding.scenario,prefix+'-inspected.private.json');
    const receipt=read(path),v=receipt.value;
    requireThat(closed(v,['schemaVersion','kind','scenario','rootRecordKey','controllerSha256','sourceSha256','claimSha256',
        'runID','reservationID','frame','inspection'])&&v.schemaVersion===1&&v.kind==='STARSECTOR_SEED_WIZARD_CONTROL'&&
        v.scenario===binding.scenario&&v.rootRecordKey===binding.campaign.rootClock.recordKey&&v.controllerSha256===binding.controllerSha256&&
        v.sourceSha256===binding.sourceSha256&&v.claimSha256===read(resolve(binding.scenario,'guard-'+binding.batch+'-consumed.private.json')).sha256&&
        v.runID===binding.runID&&v.reservationID===previous.reservationID,'seed wizard inspection lineage changed');
    const {window}=original(binding,previous,v.frame,prefix,now,after),i=v.inspection;
    requireThat(closed(i,['issuer','driver','inspectedAtMs','state','controlLabel','enabled','coordinate','generatedName'])&&
        i.issuer==='/root'&&i.driver==='/root/recovery_game_driver'&&Number.isSafeInteger(i.inspectedAtMs)&&
        i.inspectedAtMs>=v.frame.capturedAtMs&&i.inspectedAtMs<=now&&i.inspectedAtMs<=after?.observedAtMs&&
        i.state===row[1]&&i.controlLabel===row[2]&&i.enabled===true&&
        (binding.batch==='E1'&&ordinal===2?i.generatedName==='':validName(i.generatedName))&&
        (binding.batch!=='E2'||i.generatedName===binding.e1WizardReceipt.generatedName)&&
        Array.isArray(i.coordinate)&&i.coordinate.length===2&&i.coordinate.every(Number.isSafeInteger)&&
        i.coordinate[0]>=window.bounds.x&&i.coordinate[0]<window.bounds.x+window.bounds.width&&
        i.coordinate[1]>=window.bounds.y&&i.coordinate[1]<window.bounds.y+window.bounds.height&&
        isDeepStrictEqual(i.coordinate,args.coordinate),'exact enabled seed wizard control required');
    return {ordinal,stage:row[0],inspectionSha256:receipt.sha256};
}

/** Occupation handoff retains all three E1 actions and the known provider closure.
 * @param {any} context */
export function seedWizardHandoff({binding,reference,now,initial,initialSource,claim,state,action,providerClock,alreadyTransferred}) {
    const entries=(state.resolvedActions??[]).filter((/** @type {any} */ a)=>a.batch==='E1'&&a.runID===initial.runID&&a.sourceSha256===initial.sourceSha256);
    requireThat(entries.length===2&&entries[0].wizardStep?.stage==='NEW_GAME'&&entries[1].wizardStep?.stage==='GENERATE'&&
        entries.every((/** @type {any} */ a)=>a.status==='AWAITING_AFTER'&&a.tool==='left_click')&&action?.wizardStep?.stage==='CONTINUE'&&
        action.status==='AWAITING_AFTER'&&action.tool==='left_click'&&
        isDeepStrictEqual(entries[0].args.coordinate,initial.b1MenuReceipt.newGameCoordinate),'complete seed wizard entry required');
    const path=resolve(binding.scenario,'E1-occupation-inspected.private.json');
    requireThat(closed(reference,['path','sha256'])&&reference.path===path,'fixed seed occupation handoff required');
    const receipt=read(path),v=receipt.value,i=v.inspection;
    requireThat(receipt.sha256===reference.sha256&&closed(v,['schemaVersion','kind','scenario','flow','rootRecordKey',
        'controllerSha256','e1','frame','inspection','provider'])&&v.schemaVersion===1&&v.kind==='STARSECTOR_E1_OCCUPATION_INSPECTION'&&
        v.scenario===binding.scenario&&v.flow===binding.campaign.flow&&v.rootRecordKey===binding.campaign.rootClock.recordKey&&
        v.controllerSha256===binding.controllerSha256&&closed(v.e1,['sourceSha256','claimSha256','runID','reservations'])&&
        v.e1.sourceSha256===initialSource.sha256&&v.e1.claimSha256===claim.sha256&&v.e1.runID===initial.runID&&
        isDeepStrictEqual(v.e1.reservations,[...entries,action].map(a=>a.reservationID))&&
        (!alreadyTransferred||action.menuReceiptSha256===receipt.sha256),'seed occupation handoff lineage changed');
    original(initial,action,v.frame,'E1-occupation',now);
    requireThat(closed(i,['issuer','driver','inspectedAtMs','state','generatedName'])&&i.issuer==='/root'&&
        i.driver==='/root/recovery_game_driver'&&i.state==='OCCUPATION_SELECTION'&&validName(i.generatedName)&&
        Number.isSafeInteger(i.inspectedAtMs)&&i.inspectedAtMs>=v.frame.capturedAtMs&&i.inspectedAtMs<=now,'actual occupation inspection required');
    // Revalidate the two historical per-action control receipts against their actual resolved AFTERs.
    for(const [index,previous,next] of [[0,entries[0],entries[1]],[1,entries[1],action]]) {
        const prefix='E1-wizard-'+(Number(index)+2),png=file(resolve(binding.scenario,prefix,'F1.original.png'),32*1024*1024);
        requireThat(previous.after?.pngSha256===png.sha256,'historical seed control original changed');
        const proof=seedWizardInput(initial,{resolvedActions:entries.slice(0,Number(index)),pending:previous},next.args,
            {...previous.after,pngBytes:png.bytes},now);
        requireThat(proof.inspectionSha256===previous.seedWizardInspectionSha256,'historical seed control inspection changed');
        if(Number(index)===1)requireThat(read(resolve(binding.scenario,prefix+'-inspected.private.json')).value.inspection.generatedName===i.generatedName,'generated seed name changed');
    }
    const closedAt=knownProviderClosure({binding,initial,initialSourceSha256:initialSource.sha256,proof:v,providerClock,action,now});
    return {receiptSha256:receipt.sha256,generatedName:i.generatedName,action,providerClock,closedAt,alreadyTransferred};
}
