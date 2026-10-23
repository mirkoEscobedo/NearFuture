// @ts-check
import { dirname, basename, resolve } from 'node:path';
/** Preserve canonical source identity while adding DATA-only callback locations.
 * Existing composition remains responsible for admission and native authority.
 * @param {any} profile @param {any} materialized @returns {any[]} */
export function projectCaptureSlots(profile, materialized) {
    if (!Array.isArray(profile?.batches) || !Array.isArray(materialized?.bindings) ||
        profile.batches.length !== materialized.bindings.length || !Number.isSafeInteger(profile.maximumProfileFrames))
        throw new Error('EXACT_EXISTING_PROFILE_BINDINGS_REQUIRED');
    const ids = new Set(), frames = new Set(), batchIds = new Set();
    const projected = profile.batches.map((/** @type {any} */ batch) => {
        const matches = materialized.bindings.filter((/** @type {any} */ row) => row.id === batch.compositionBindingId);
        if (matches.length !== 1 || ids.has(batch.compositionBindingId) || batchIds.has(batch.id))
            throw new Error('ONE_EXACT_BINDING_PER_BATCH_REQUIRED');
        const binding = matches[0];
        if (binding.evidenceRoot !== batch.evidenceRoot || !Array.isArray(binding.frames) || binding.frames.length > 6 ||
            !Array.isArray(batch.frameSlots) || binding.frames.length !== batch.frameSlots.length ||
            binding.frames.some((/** @type {any} */ slot, /** @type {number} */ i) => slot.id !== batch.frameSlots[i]))
            throw new Error('EXACT_DECLARED_FRAME_ORDER_REQUIRED');
        ids.add(binding.id); batchIds.add(batch.id);
        const copy = JSON.parse(JSON.stringify(binding));
        copy.batchId = batch.id;
        copy.frames = copy.frames.map((/** @type {any} */ slot) => {
            if (frames.has(slot.id)) throw new Error('UNIQUE_CANONICAL_FRAME_ID_REQUIRED');
            frames.add(slot.id);
            const name = basename(slot.png);
            if (!/^F[1-9][0-9]{0,2}\.original\.png$/.test(name)) throw new Error('DECLARED_PNG_BASENAME_REQUIRED');
            const stem = name.slice(0, -'.original.png'.length);
            return { ...slot, intentPath: resolve(dirname(slot.png), stem + '.capture-intent.private.json'),
                callbackReceipt: resolve(dirname(slot.png), stem + '.callback-completion.private.json') };
        });
        return copy;
    });
    if (frames.size > profile.maximumProfileFrames) throw new Error('EXISTING_PROFILE_FRAME_CAP_REQUIRED');
    return projected;
}
