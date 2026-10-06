// @ts-check
// Architectural lint, not a sandbox for adversarial source code.
/** @typedef {{ name: string, req?: string, uses_default_features?: boolean, kind?: string | null, path?: string }} Dependency */
/** @typedef {{ name: string, dependencies: Dependency[] }} Package */
/** @typedef {{ path: string, content: string }} Source */

/** @param {string} content */
function codeOnly(content) {
    return content.replace(/\/\*[\s\S]*?\*\/|\/\/[^\n]*|"(?:\\.|[^"\\])*"/g, '');
}

/** @param {Package[]} packages @param {Source[]} sources @returns {string[]} */
export function dependencyViolations(packages, sources) {
    const violations = [];
    const pureDependencies = new Map([['sha2', '=0.10.9'], ['unicode-normalization', '=0.1.25'], ['ed25519-dalek', '=2.2.0']]);
    for (const pkg of packages.filter(pkg => ['nf-contract', 'nf-kernel'].includes(pkg.name))) {
        for (const dependency of pkg.dependencies.filter(dependency => dependency.kind !== 'dev')) {
            const kernelContract = pkg.name === 'nf-kernel' && dependency.name === 'nf-contract' &&
                typeof dependency.path === 'string' && dependency.path.replaceAll('\\', '/').endsWith('/crates/nf-contract');
            const pureHash = dependency.name === 'sha2' && dependency.req === '=0.10.9' && dependency.uses_default_features === false;
            const permitted = pkg.name === 'nf-kernel' ? kernelContract || pureHash :
                pureDependencies.get(dependency.name) === dependency.req && dependency.uses_default_features === false;
            if (!permitted) {
                violations.push(`${pkg.name}: domain dependency ${dependency.name} is forbidden`);
            }
        }
    }
    for (const { path, content } of sources) {
        const code = codeOnly(content);
        if (['crates/nf-contract/', 'crates/nf-kernel/'].some(prefix => path.startsWith(prefix))) {
            if (/\bstd\s*::|\bextern\s+crate\s+std\b/.test(code)) {
                violations.push(`${path}: Rust domain must not access std`);
            }
            if (path.endsWith('/lib.rs') && !/#!\s*\[\s*no_std\s*\]/.test(code)) {
                violations.push(`${path}: Rust domain requires no_std`);
            }
        }
        if (path.startsWith('java/contract/src/main/') && /\b(?:java\s*\.\s*(?:io|net|sql|time)\b|java\s*\.\s*nio\s*\.\s*(?:file|channels)\b|java\s*\.\s*util\s*\.\s*concurrent\b|java\s*\.\s*lang\s*\.\s*reflect\b|com\s*\.\s*fs\s*\.\s*starfarer\b|org\s*\.\s*(?:lwjgl|apache|sqlite)\b|System\s*\.\s*(?!arraycopy\b)|Runtime|Thread|ProcessBuilder|ClassLoader)\b/.test(code)) {
            violations.push(`${path}: Java contract may not access game or ambient effects`);
        }
    }
    return violations;
}
