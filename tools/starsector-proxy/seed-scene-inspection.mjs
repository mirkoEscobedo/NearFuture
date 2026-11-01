// @ts-check
import { existsSync } from 'node:fs';
import { resolve } from 'node:path';
import { isDeepStrictEqual } from 'node:util';
import { completePng } from './direct-frames.mjs';
import { requireThat, file, read, closed } from './campaign-proof.mjs';
import { isSeedApplicationQuit, isSeedApplicationQuitConfirmation } from './seed-application-quit-profile.mjs';

const scenes = Object.freeze({
    paused: { prefix: 'E1-seed-paused', label: 'paused seed', batch: 'E1',
        kind: 'STARSECTOR_SEED_PAUSE_INSPECTION', state: 'INTENDED_DISPOSABLE_SEED_PAUSED' },
    'exit-confirmation': { prefix: 'E2-exit-confirmation', label: 'exit confirmation', batch: 'E2',
        kind: 'STARSECTOR_EXIT_CONFIRMATION_INSPECTION', state: 'EXIT_GAME_CONFIRMATION',
        dialogText: 'Exit game?', button: 'Yes', coordinate: 'confirmCoordinate', buttonField: 'confirmLabel' },
    'no-save-return': { prefix: 'E2-no-save-return-confirmation', label: 'no-save return', batch: 'E2',
        kind: 'STARSECTOR_NO_SAVE_RETURN_CONFIRMATION_INSPECTION', state: 'RETURN_TO_MAIN_MENU_WITHOUT_SAVING',
        dialogText: 'This will exit to the main menu WITHOUT saving your current game.',
        button: 'Ok', coordinate: 'confirmCoordinate', buttonField: 'confirmLabel' },
    'returned-main-menu': { prefix: 'E2-returned-main-menu', label: 'returned main menu', batch: 'E2',
        kind: 'STARSECTOR_RETURNED_MAIN_MENU_INSPECTION', state: 'STARSECTOR_MAIN_MENU_WITH_QUIT',
        button: 'Quit', coordinate: 'quitCoordinate', buttonField: 'quitLabel' },
    'application-quit-confirmation': { prefix: 'E2-application-quit-confirmation', label: 'application Quit confirmation', batch: 'E2',
        kind: 'STARSECTOR_APPLICATION_QUIT_CONFIRMATION_INSPECTION', state: 'EXIT_GAME_CONFIRMATION',
        dialogText: 'Exit game?', button: 'Yes', coordinate: 'confirmCoordinate', buttonField: 'confirmLabel' },
});

/** Read-only proof: no ledger updates, claims or authority renewal.
 * The driver inspects the original; a capture receipt alone does not prove a scene.
 * @param {any} binding @param {any} previous @param {any} after @param {number} now
 * @param {keyof typeof scenes} scene @param {any} [args] */
export function inspectSeedScene(binding, previous, after, now, scene, args) {
    const definition = scenes[scene];
    requireThat(definition && binding.batch === definition.batch, 'matching seed scene batch required');
    const applicationQuit = scene === 'no-save-return' || scene === 'returned-main-menu';
    requireThat(!applicationQuit || isSeedApplicationQuit(binding), 'application Quit scene profile required');
    if (scene === 'application-quit-confirmation') requireThat(isSeedApplicationQuitConfirmation(binding),
        'application Quit confirmation profile required');
    if (scene === 'exit-confirmation') requireThat(!isSeedApplicationQuit(binding) &&
        closed(binding.exitConfirmation, ['schemaVersion', 'dialogText', 'confirmLabel']) &&
        binding.exitConfirmation.schemaVersion === 1 && binding.exitConfirmation.dialogText === 'Exit game?' &&
        binding.exitConfirmation.confirmLabel === 'Yes', 'matching declared exit confirmation required');
    const { prefix, label } = definition;
    const path = resolve(binding.scenario, prefix + '-inspected.private.json');
    requireThat(existsSync(path), label + ' inspection required before input');
    const receipt = read(path), value = receipt.value;
    const original = read(resolve(binding.scenario, 'plan.json')).value.seed;
    requireThat(closed(value, ['schemaVersion', 'kind', 'scenario', 'rootRecordKey', 'controllerSha256',
        'sourceSha256', 'claimSha256', 'runID', 'reservationID', 'manifestSha256', 'stagingReceiptSha256', 'frame', 'inspection']) &&
        value.schemaVersion === 1 && value.kind === definition.kind && value.scenario === binding.scenario &&
        value.rootRecordKey === binding.campaign.rootClock.recordKey && value.controllerSha256 === binding.controllerSha256 &&
        value.sourceSha256 === binding.sourceSha256 && value.runID === binding.runID && value.reservationID === previous.reservationID &&
        value.claimSha256 === read(resolve(binding.scenario, 'guard-' + binding.batch + '-consumed.private.json')).sha256 &&
        value.manifestSha256 === original.manifestSha256?.toLowerCase() &&
        value.stagingReceiptSha256 === original.stagingReceiptSha256?.toLowerCase(), label + ' inspection lineage changed');
    const frame = value.frame, inspection = value.inspection;
    const pngPath = resolve(binding.scenario, prefix, 'F1.original.png');
    const metadataPath = resolve(binding.scenario, prefix, 'capture-001.metadata.private.json');
    const rawPath = resolve(binding.scenario, prefix, 'capture-001.raw.private.json');
    requireThat(closed(frame, ['pngPath', 'pngSha256', 'metadataPath', 'metadataSha256', 'capturedAtMs']) &&
        frame.pngPath === pngPath && frame.metadataPath === metadataPath, label + ' original paths changed');
    const png = file(pngPath, 32 * 1024 * 1024), metadata = read(metadataPath), m = metadata.value;
    const raw = read(rawPath, 32 * 1024 * 1024);
    const images = raw.value.content?.filter((/** @type {any} */ row) => row.type === 'image');
    const dimensions = completePng(png.bytes);
    requireThat(png.sha256 === frame.pngSha256 && metadata.sha256 === frame.metadataSha256 && png.bytes.equals(after.pngBytes) &&
        raw.value.isError !== true && images?.length === 1 && images[0].mimeType === 'image/png' &&
        png.bytes.toString('base64') === images[0].data && m.captureReceipt === true &&
        m.rawPath === rawPath && m.rawSha256 === raw.sha256 && m.pngPath === pngPath && m.pngSha256 === png.sha256 &&
        m.runID === binding.runID && m.batch === binding.batch && m.sourceSha256 === binding.sourceSha256 &&
        m.reservationID === previous.reservationID && m.capturedAtMs === frame.capturedAtMs &&
        m.args?.target_window_id === binding.campaign.windowId && m.args.quality === 0 && m.args.width === after.window.bounds.width &&
        m.window?.windowId === after.window.windowId && m.window.pid === after.window.pid && m.window.isFocused === true &&
        isDeepStrictEqual(m.window.bounds, after.window.bounds) && isDeepStrictEqual(m.dimensions, dimensions) &&
        dimensions.width === after.window.bounds.width && dimensions.height === after.window.bounds.height &&
        Number.isSafeInteger(frame.capturedAtMs) && frame.capturedAtMs >= after.capturedAtMs && frame.capturedAtMs <= after.observedAtMs,
    label + ' original custody changed');
    const button = 'button' in definition ? definition : undefined;
    requireThat(closed(inspection, ['issuer', 'driver', 'inspectedAtMs', 'state',
        ...(button ? [button.buttonField, button.coordinate, ...('dialogText' in button ? ['dialogText'] : [])] : [])]) &&
        inspection.issuer === '/root' && inspection.driver === '/root/recovery_game_driver' &&
        inspection.state === definition.state && Number.isSafeInteger(inspection.inspectedAtMs) &&
        inspection.inspectedAtMs >= frame.capturedAtMs && inspection.inspectedAtMs <= after.observedAtMs &&
        inspection.inspectedAtMs <= now, label + ' visible inspection required');
    if (button) requireThat((!('dialogText' in button) || inspection.dialogText === button.dialogText) &&
        inspection[button.buttonField] === button.button && Array.isArray(inspection[button.coordinate]) &&
        inspection[button.coordinate].length === 2 &&
        inspection[button.coordinate].every((/** @type {any} */ value) => Number.isSafeInteger(value)) &&
        isDeepStrictEqual(inspection[button.coordinate], args?.coordinate), scene === 'exit-confirmation' ?
            'matching declared exit confirmation required' : 'matching ' + label + ' control required');
    return receipt.sha256;
}
