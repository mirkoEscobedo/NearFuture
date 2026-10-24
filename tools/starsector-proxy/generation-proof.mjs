// @ts-check
import {resolve} from 'node:path';
import {isDeepStrictEqual} from 'node:util';
import {requireThat,file,read,closed,knownProviderClosure} from './campaign-proof.mjs';
import {completePng} from './direct-frames.mjs';

/** Complete original/raw/metadata custody for one fixed generated-name scene.
 * No scene claim is inferred from capture success. @param {any} context */
export function generationOriginal({binding,action,frame,prefix,now,after}) {
    const pngPath=resolve(binding.scenario,prefix,'F1.original.png');
    const metadataPath=resolve(binding.scenario,prefix,'capture-001.metadata.private.json');
    const rawPath=resolve(binding.scenario,prefix,'capture-001.raw.private.json');
    requireThat(closed(frame,['pngPath','pngSha256','metadataPath','metadataSha256','capturedAtMs'])&&
        frame.pngPath===pngPath&&frame.metadataPath===metadataPath,'generation original paths changed');
    const png=file(pngPath,32*1024*1024),metadata=read(metadataPath),m=metadata.value;
    const raw=read(rawPath,32*1024*1024),images=raw.value.content?.filter((/** @type {any} */ row)=>row.type==='image');
    const dimensions=completePng(png.bytes);
    requireThat(frame.pngSha256===png.sha256&&frame.metadataSha256===metadata.sha256&&
        raw.value.isError!==true&&images?.length===1&&images[0].mimeType==='image/png'&&
        typeof images[0].data==='string'&&png.bytes.toString('base64')===images[0].data&&
        m.captureReceipt===true&&m.rawPath===rawPath&&m.rawSha256===raw.sha256&&
        m.pngPath===pngPath&&m.pngSha256===png.sha256&&m.runID===binding.runID&&m.batch==='E1'&&
        m.sourceSha256===binding.sourceSha256&&m.reservationID===action.reservationID&&
        m.capturedAtMs===frame.capturedAtMs&&m.args?.target_window_id===binding.campaign.windowId&&
        m.args.quality===0&&m.args.width===m.window?.bounds?.width&&m.window.windowId===binding.campaign.windowId&&
        binding.ownedPids.includes(m.window.pid)&&m.window.isFocused===true&&isDeepStrictEqual(m.dimensions,dimensions)&&
        dimensions.width===m.window.bounds.width&&dimensions.height===m.window.bounds.height&&
        Number.isSafeInteger(action.knownResultAtMs)&&Number.isSafeInteger(frame.capturedAtMs)&&
        frame.capturedAtMs>=action.knownResultAtMs&&frame.capturedAtMs<=now,'generation original custody changed');
    if(after)requireThat(png.bytes.equals(after.pngBytes)&&isDeepStrictEqual(m.window,after.window)&&
        frame.capturedAtMs>=after.capturedAtMs&&frame.capturedAtMs<=after.observedAtMs,
    'current generation original changed');
    return {window:m.window,png};
}

/** @param {any} inspection @param {any} frame @param {number} now */
function inspected(inspection,frame,now) {
    requireThat(inspection.issuer==='/root'&&inspection.driver==='/root/recovery_game_driver'&&
        Number.isSafeInteger(inspection.inspectedAtMs)&&inspection.inspectedAtMs>=frame.capturedAtMs&&
        inspection.inspectedAtMs<=now,'generation visible inspection required');
}

/** @param {any} coordinate @param {any} window */
export function generationCoordinate(coordinate,window) {
    requireThat(Array.isArray(coordinate)&&coordinate.length===2&&coordinate.every(Number.isSafeInteger)&&
        coordinate[0]>=window.bounds.x&&coordinate[0]<window.bounds.x+window.bounds.width&&
        coordinate[1]>=window.bounds.y&&coordinate[1]<window.bounds.y+window.bounds.height,
    'generation inspected control coordinate required');
}

/** Proves the one Generate action after New Game. It neither reserves nor dispatches.
 * @param {any} binding @param {any} previous @param {any} after @param {number} now @param {any} args */
export function inspectGenerationControl(binding,previous,after,now,args) {
    const receipt=read(resolve(binding.scenario,'E1-generate-inspected.private.json')),value=receipt.value;
    requireThat(binding.schemaVersion===5&&binding.batch==='E1'&&closed(value,['schemaVersion','kind','scenario',
        'rootRecordKey','controllerSha256','sourceSha256','claimSha256','runID','reservationID','frame','inspection'])&&
        value.schemaVersion===1&&value.kind==='STARSECTOR_NEW_GAME_GENERATE_INSPECTION'&&value.scenario===binding.scenario&&
        value.rootRecordKey===binding.campaign.rootClock.recordKey&&value.controllerSha256===binding.controllerSha256&&
        value.sourceSha256===binding.sourceSha256&&value.runID===binding.runID&&value.reservationID===previous.reservationID&&
        value.claimSha256===read(resolve(binding.scenario,'guard-E1-consumed.private.json')).sha256,
    'generation control inspection lineage changed');
    const {window}=generationOriginal({binding,action:previous,frame:value.frame,prefix:'E1-generate',now,after});
    const inspection=value.inspection;
    requireThat(closed(inspection,['issuer','driver','inspectedAtMs','state','nameBlank','generateLabel','generateCoordinate'])&&
        inspection.state==='NEW_GAME_IDENT_CHIP_BLANK_NAME'&&inspection.nameBlank===true&&inspection.generateLabel==='GENERATE',
    'blank-name Generate inspection required');
    inspected(inspection,value.frame,now);generationCoordinate(inspection.generateCoordinate,window);
    requireThat(isDeepStrictEqual(args.coordinate,inspection.generateCoordinate),'only the inspected Generate coordinate is qualified');
    return receipt.sha256;
}

/** Read-only proof of the Generate result and complete E1 closure. No clock is renewed.
 * @param {any} context */
export function generatedNameHandoff({binding,reference,now,initial,initialSource,claim,state,action,providerClock,alreadyTransferred}) {
    const entry=state.resolvedActions?.filter((/** @type {any} */ row)=>row.batch==='E1'&&
        row.runID===initial.runID&&row.sourceSha256===initial.sourceSha256);
    requireThat(entry?.length===1&&entry[0].tool==='left_click'&&entry[0].status==='AWAITING_AFTER'&&
        isDeepStrictEqual(entry[0].args?.coordinate,initial.b1MenuReceipt.newGameCoordinate)&&
        entry[0].args.target_window_id===initial.campaign.windowId&&action.tool==='left_click'&&
        action.args?.target_window_id===initial.campaign.windowId,'original New Game and Generate reservations required');
    const original=file(resolve(binding.scenario,'E1-generate/F1.original.png'),32*1024*1024);
    requireThat(entry[0].after?.pngSha256===original.sha256&&
        inspectGenerationControl(initial,entry[0],{...entry[0].after,pngBytes:original.bytes},now,action.args)===entry[0].generateInspectionSha256,
    'historical Generate inspection changed');
    const path=resolve(binding.scenario,'E1-generated-name-inspected.private.json');
    requireThat(closed(reference,['path','sha256'])&&reference.path===path,'generated-name proof reference required');
    const receipt=read(path),value=receipt.value;
    requireThat(receipt.sha256===reference.sha256&&closed(value,['schemaVersion','kind','scenario','flow','rootRecordKey',
        'controllerSha256','e1','frame','inspection','provider'])&&value.schemaVersion===1&&
        value.kind==='STARSECTOR_E1_GENERATED_NAME_INSPECTION'&&value.scenario===binding.scenario&&value.flow===binding.campaign.flow&&
        value.rootRecordKey===binding.campaign.rootClock.recordKey&&value.controllerSha256===binding.controllerSha256&&
        closed(value.e1,['sourceSha256','claimSha256','runID','newGameReservationID','generateReservationID'])&&
        value.e1.sourceSha256===initialSource.sha256&&value.e1.claimSha256===claim.sha256&&value.e1.runID===initial.runID&&
        value.e1.newGameReservationID===entry[0].reservationID&&value.e1.generateReservationID===action.reservationID&&
        (!alreadyTransferred||action.menuReceiptSha256===receipt.sha256),'generated-name reservation proof changed');
    const {window}=generationOriginal({binding:initial,action,frame:value.frame,prefix:'E1-generated-name',now});
    const inspection=value.inspection;
    requireThat(closed(inspection,['issuer','driver','inspectedAtMs','state','generatedName','continueLabel','continueCoordinate'])&&
        inspection.state==='NEW_GAME_IDENT_CHIP_GENERATED_NAME'&&typeof inspection.generatedName==='string'&&
        inspection.generatedName.trim().length>0&&inspection.generatedName.length<=128&&
        !/[\u0000-\u001f\u007f]/u.test(inspection.generatedName)&&inspection.continueLabel==='1. Continue',
    'generated name and first Continue inspection required');
    inspected(inspection,value.frame,now);generationCoordinate(inspection.continueCoordinate,window);
    const closedAt=knownProviderClosure({binding,initial,initialSourceSha256:initialSource.sha256,
        proof:value,providerClock,action,now});
    return {receiptSha256:receipt.sha256,continueCoordinate:inspection.continueCoordinate,action,providerClock,closedAt,alreadyTransferred};
}
