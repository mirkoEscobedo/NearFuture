// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import { dependencyViolations } from '../../tools/architecture/boundaries.mjs';

test('domain rejects dependencies on a database adapter', () => {
    const packages = [{ name: 'nf-contract', dependencies: [{ name: 'rusqlite' }] }];
    assert.deepEqual(dependencyViolations(packages, []), ['nf-contract: domain dependency rusqlite is forbidden']);
});

test('domain rejects standard-library network access', () => {
    const sources = [{ path: 'crates/nf-contract/src/lib.rs', content: '#![no_std]\nextern crate std;\nuse std::net::TcpStream;' }];
    assert.deepEqual(dependencyViolations([], sources), ['crates/nf-contract/src/lib.rs: Rust domain must not access std']);
});

test('Java contract rejects game and ambient I/O APIs', () => {
    for (const imported of ['com.fs.starfarer.api.Global', 'java.net.Socket', 'java.sql.Connection', 'java.lang.reflect.Method']) {
        const sources = [{ path: 'java/contract/src/main/java/nf/contract/Bad.java', content: `import ${imported};` }];
        assert.deepEqual(dependencyViolations([], sources), ['java/contract/src/main/java/nf/contract/Bad.java: Java contract may not access game or ambient effects']);
    }
});


test('reviewed deterministic dependencies are permitted only with pinned no_std settings', () => {
    const dependencies = [
        { name: 'sha2', req: '=0.10.9', uses_default_features: false },
        { name: 'unicode-normalization', req: '=0.1.25', uses_default_features: false },
    ];
    assert.deepEqual(dependencyViolations([{ name: 'nf-contract', dependencies }], []), []);
    assert.deepEqual(dependencyViolations([{ name: 'nf-contract', dependencies: [{ ...dependencies[0], uses_default_features: true }] }], []), ['nf-contract: domain dependency sha2 is forbidden']);
});

test('test-only fixture readers and pure byte buffers remain outside production effects', () => {
    const packages = [{ name: 'nf-contract', dependencies: [{ name: 'serde_json', kind: 'dev' }] }];
    const sources = [{ path: 'java/contract/src/main/java/nf/contract/Pure.java', content: 'import java.nio.charset.StandardCharsets; System.arraycopy(source, 0, target, 0, 1);' }];
    assert.deepEqual(dependencyViolations(packages, sources), []);
    for (const imported of ['java.nio.file.Files', 'java.nio.channels.SocketChannel']) {
        assert.equal(dependencyViolations([], [{ path: sources[0].path, content: `import ${imported};` }]).length, 1);
    }
    assert.equal(dependencyViolations([], [{ path: sources[0].path, content: 'System.currentTimeMillis();' }]).length, 1);
});

test('world kernel rejects effect dependencies and ambient std access', () => {
    const dependencies = [{ name: 'rusqlite' }];
    assert.deepEqual(dependencyViolations([{ name: 'nf-kernel', dependencies }], []), ['nf-kernel: domain dependency rusqlite is forbidden']);
    const source = {path: 'crates/nf-kernel/src/lib.rs', content: '#![no_std]\nuse std::time::SystemTime;'};
    assert.deepEqual(dependencyViolations([], [source]), ['crates/nf-kernel/src/lib.rs: Rust domain must not access std']);
});

test('kernel accepts the local contract and exact pure hash while rejecting registry substitution', () => {
    const hash = {name: 'sha2', req: '=0.10.9', uses_default_features: false};
    for (const path of ['E:/checkout/crates/nf-contract', 'E:\\checkout\\crates\\nf-contract']) {
        assert.deepEqual(dependencyViolations([{name: 'nf-kernel', dependencies: [hash, {name: 'nf-contract', path}]}], []), []);
    }
    assert.deepEqual(dependencyViolations([{name: 'nf-kernel', dependencies: [{name: 'nf-contract', req: '^0.1.0'}]}], []), ['nf-kernel: domain dependency nf-contract is forbidden']);
});

test('Nex boundary permits only the local pure contract and refuses effects', () => {
    const pkg = { name: 'nf-nex-boundary', dependencies: [{ name: 'rusqlite' }] };
    assert.deepEqual(dependencyViolations([pkg], []), ['nf-nex-boundary: domain dependency rusqlite is forbidden']);
    assert.deepEqual(dependencyViolations([], [{path: 'crates/nf-nex-boundary/src/lib.rs', content: '#![no_std]\nuse std::fs::File;'}]), ['crates/nf-nex-boundary/src/lib.rs: Rust domain must not access std']);
    assert.deepEqual(dependencyViolations([], [{path: 'crates/nf-nex-boundary/src/lib.rs', content: 'pub struct View;'}]), ['crates/nf-nex-boundary/src/lib.rs: Rust domain requires no_std']);
    const local = { name: 'nf-contract', path: '/checkout/crates/nf-contract' };
    assert.deepEqual(dependencyViolations([{name: pkg.name, dependencies: [local]}], []), []);
    for (const dependency of [{name: 'nf-contract', req: '^0.1.0'}, {name: 'sha2', req: '=0.10.9', uses_default_features: false}]) {
        assert.deepEqual(dependencyViolations([{name: pkg.name, dependencies: [dependency]}], []), [`${pkg.name}: domain dependency ${dependency.name} is forbidden`]);
    }
});

test('Nex shadow permits only copied local domains and exact pure hash', () => {
    const name = 'nf-nex-shadow';
    const hash = {name: 'sha2', req: '=0.10.9', uses_default_features: false};
    const contract = {name: 'nf-contract', path: '/checkout/crates/nf-contract'};
    const boundary = {name: 'nf-nex-boundary', path: 'E:\\checkout\\crates\\nf-nex-boundary'};
    assert.deepEqual(dependencyViolations([{name, dependencies: [contract, boundary, hash]}], []), []);
    for (const dependency of [{name: 'nf-store'}, {name: 'nf-wire'}, {name: 'nf-nex-boundary', req: '^0.1.0'}, {...hash, uses_default_features: true}]) {
        assert.deepEqual(dependencyViolations([{name, dependencies: [dependency]}], []), [`${name}: domain dependency ${dependency.name} is forbidden`]);
    }
    assert.deepEqual(dependencyViolations([], [{path: 'crates/nf-nex-shadow/src/lib.rs', content: '#![no_std]\nuse std::net::TcpStream;'}]), ['crates/nf-nex-shadow/src/lib.rs: Rust domain must not access std']);
});
test('miniature World core accepts only its local contract and maintained pure hash', () => {
    const name = 'nf-world';
    const hash = {name: 'sha2', req: '=0.10.9', uses_default_features: false};
    const contract = {name: 'nf-contract', path: '/checkout/crates/nf-contract'};
    assert.deepEqual(dependencyViolations([{name, dependencies: [contract, hash]}], []), []);
    for (const dependency of [{name: 'nf-store'}, {name: 'nf-kernel'}, {name: 'nf-contract', req: '^0.1.0'}, {...hash, uses_default_features: true}]) {
        assert.deepEqual(dependencyViolations([{name, dependencies: [dependency]}], []), [`${name}: domain dependency ${dependency.name} is forbidden`]);
    }
    assert.deepEqual(dependencyViolations([], [{path: 'crates/nf-world/src/lib.rs', content: '#![no_std]\nuse std::time::SystemTime;'}]), ['crates/nf-world/src/lib.rs: Rust domain must not access std']);
});

test('miniature kernel admits its local pure World core but rejects registry replacement and effects', () => {
    const name = 'nf-kernel';
    for (const path of ['/checkout/crates/nf-world', 'E:\\checkout\\crates\\nf-world']) {
        assert.deepEqual(dependencyViolations([{name, dependencies: [{name: 'nf-world', path}]}], []), []);
    }
    for (const dependency of [{name: 'nf-world', req: '^0.1.0'}, {name: 'nf-world', path: '/checkout/crates/other'}, {name: 'nf-store'}, {name: 'getrandom'}]) {
        assert.deepEqual(dependencyViolations([{name, dependencies: [dependency]}], []), [`${name}: domain dependency ${dependency.name} is forbidden`]);
    }
});
