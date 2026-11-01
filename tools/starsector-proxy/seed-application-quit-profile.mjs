// @ts-check
export const seedApplicationQuitFlow = 'DISPOSABLE_SEED_RETURN_MENU_QUIT_NO_SAVE';

/** A phase still needs the matching immutable root control at admission.
 * @param {any} binding */
export const isSeedApplicationQuit = binding => binding?.schemaVersion === 3 &&
    binding.campaign?.flow === seedApplicationQuitFlow;
