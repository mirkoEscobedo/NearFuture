// @ts-check
import assert from 'node:assert/strict';
import test from 'node:test';
import { spawnSync } from 'node:child_process';
import { randomUUID } from 'node:crypto';
import { existsSync, rmSync, readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { fixture } from './batch-test-fixture.mjs';

const runner = new URL('./run.mjs', import.meta.url).href;
const root = fileURLToPath(new URL('../../', import.meta.url));

// Replace only the external CLI boundary; execute the real adapter and its CLI.
const boundary = `
import cp from 'node:child_process';
import { syncBuiltinESMExports } from 'node:module';
import { EventEmitter } from 'node:events';
import { PassThrough } from 'node:stream';
const cliDirectory = options => {
  const previous = process.cwd();
  try { process.chdir(options.env.PWD ?? options.cwd); return process.cwd(); }
  finally { process.chdir(previous); }
};
cp.spawn = (_program, args, options) => {
  console.log('EXTERNAL_COMMAND:' + args[0]);
  if (args[0] === 'api') console.log('API_CONTEXT:' + JSON.stringify({
    directory: args.find(arg => arg.startsWith('location[directory]='))?.slice('location[directory]='.length),
    cliDirectory: cliDirectory(options),
  }));
  if (args.includes('experimental.mcp.add')) {
    const config = JSON.parse(args[args.indexOf('--data') + 1]).config;
    console.log('REGISTERED_BATCH:' + JSON.stringify({ command: config.command,
      authorization: config.environment.NF_PROXY_BATCH_AUTHORIZATION ?? null,
      runID: config.environment.NF_PROXY_BATCH_RUN_ID ?? null }));
  }
  if (args[0] === 'run') console.log('RUN_CONTEXT:' + JSON.stringify(cliDirectory(options)));
  const child = new EventEmitter();
  child.stdout = new PassThrough(); child.stderr = new PassThrough(); child.stdin = new PassThrough();
  if (args[0] === 'serve') {
    const overlay = JSON.parse(options.env.OPENCODE_CONFIG_CONTENT);
    console.log('REQUESTED_BODY:' + JSON.stringify(overlay.providers?.['unsloth-studio']?.models?.['unsloth/Qwen3.8-27B-GGUF:UD-Q4_K_XL']?.body ?? null));
    child.stdin.on('finish', () => child.emit('close', 0, null));
    setImmediate(() => child.stdout.write('{"url":"http://127.0.0.1:45678"}\\n'));
  } else setImmediate(() => {
    if (args[0] === 'api') child.stdout.write(JSON.stringify(args.includes('mcp.list')
      ? {data:[{name:'computer_use',status:{status:'connected'}},
        {name:'chrome-devtools',status:{status:'disabled'}},
        {name:'debugMcp',status:{status:'disabled'}},
        {name:'dart',status:{status:'disabled'}}]} : {data:{}}));
    if (args[0] === 'run') {
      console.log('REQUESTED_MODEL:' + args[args.indexOf('--model') + 1]);
      console.log('REQUESTED_AGENT:' + args[args.indexOf('--agent') + 1]);
    }
    child.stdout.end(); child.stderr.end(); child.emit('close', 0, null);
  });
  return child;
};
syncBuiltinESMExports();
const runnerUrl = process.argv.splice(1, 1)[0];
await import(runnerUrl);
`;

for (const [name, args, expected] of /** @type {[string, string[], string][]} */ ([
    ['default model', [], 'opencode-go/glm-5.3-flash'],
    ['explicit authorized free model', ['--model', 'opencode/deepseek-v4.1-flash-free'], 'opencode/deepseek-v4.1-flash-free'],
    ['explicit authorized local model', ['--model', 'unsloth-studio/unsloth/Qwen3.8-27B-GGUF:UD-Q4_K_XL'], 'unsloth-studio/unsloth/Qwen3.8-27B-GGUF:UD-Q4_K_XL'],
    ['explicit local inference control', ['--model', 'unsloth-studio/unsloth/Qwen3.8-27B-GGUF:UD-Q4_K_XL', '--disable-thinking'], 'unsloth-studio/unsloth/Qwen3.8-27B-GGUF:UD-Q4_K_XL'],
    ['explicit local serial-call control', ['--model', 'unsloth-studio/unsloth/Qwen3.8-27B-GGUF:UD-Q4_K_XL', '--disable-parallel-tool-calls'], 'unsloth-studio/unsloth/Qwen3.8-27B-GGUF:UD-Q4_K_XL'],
    ['both independent local controls', ['--model', 'unsloth-studio/unsloth/Qwen3.8-27B-GGUF:UD-Q4_K_XL', '--disable-thinking', '--disable-parallel-tool-calls'], 'unsloth-studio/unsloth/Qwen3.8-27B-GGUF:UD-Q4_K_XL'],
])) {
    test(name + ' reaches the external CLI without changing the agent', t => {
        const id = 'model-test-' + randomUUID();
        t.after(() => rmSync(new URL('../../.tmp/starsector-proxy/' + id, import.meta.url), { recursive: true, force: true }));
        const result = spawnSync(process.execPath, ['--input-type=module', '-e', boundary, runner,
            '--opencode', 'external-cli-boundary', '--run-id', id, ...args], { cwd: root, encoding: 'utf8', timeout: 10000 });
        assert.equal(result.error, undefined);
        assert.match(result.stdout, new RegExp('REQUESTED_MODEL:' + expected.replaceAll('.', '\\.')), result.stderr);
        assert.match(result.stdout, /REQUESTED_AGENT:starsector-observer/);
        const expectedBody = args.includes('--disable-parallel-tool-calls')
            ? (args.includes('--disable-thinking') ? { enable_thinking: false, parallel_tool_calls: false } : { parallel_tool_calls: false })
            : (args.includes('--disable-thinking') ? { enable_thinking: false } : null);
        const requested = result.stdout.match(/REQUESTED_BODY:(.+)/); assert.ok(requested);
        assert.deepEqual(JSON.parse(requested[1]), expectedBody);
        // No session or frame is fabricated by this boundary fixture.
        assert.equal(result.status, 1);
        assert.match(result.stderr, /No valid OpenCode session ID/);
    });
}

for (const args of [[], ['--model', 'unsloth-studio/unsloth/Qwen3.8-27B-GGUF:UD-Q4_K_XL', '--session-export', 'ses_historical']]) {
    test('serial-call control refuses unsupported model or historical export before effects: ' + JSON.stringify(args), () => {
        const id = 'model-test-' + randomUUID();
        const result = spawnSync(process.execPath, ['--input-type=module', '-e', boundary, runner,
            '--opencode', 'external-cli-boundary', '--run-id', id, '--disable-parallel-tool-calls', ...args],
            { cwd: root, encoding: 'utf8', timeout: 10000 });
        assert.equal(result.status, 1);
        assert.match(result.stderr, /Inference control requires a new run of the qualified local model/);
        assert.doesNotMatch(result.stdout, /EXTERNAL_COMMAND:/);
        assert.equal(existsSync(new URL('../../.tmp/starsector-proxy/' + id, import.meta.url)), false);
    });
}

test('both local controls are accurately recorded by no-model MCP readiness', t => {
    const id = 'model-test-' + randomUUID();
    const evidence = new URL('../../.tmp/starsector-proxy/' + id + '/', import.meta.url);
    t.after(() => rmSync(evidence, { recursive: true, force: true }));
    const result = spawnSync(process.execPath, ['--input-type=module', '-e', boundary, runner,
        '--opencode', 'external-cli-boundary', '--run-id', id, '--check-mcp-only', '--disable-thinking',
        '--disable-parallel-tool-calls', '--model', 'unsloth-studio/unsloth/Qwen3.8-27B-GGUF:UD-Q4_K_XL'],
        { cwd: root, encoding: 'utf8', timeout: 10000 });
    assert.equal(result.status, 0, result.stderr);
    const report = JSON.parse(readFileSync(new URL('mcp-readiness.json', evidence), 'utf8'));
    assert.deepEqual(report.requestedBody, { enable_thinking: false, parallel_tool_calls: false });
    assert.equal(report.modelDispatched, false);
    assert.doesNotMatch(result.stdout, /REQUESTED_MODEL:/);
});

test('MCP registration and readiness share the model client exact OS working directory', t => {
    const id = 'model-test-' + randomUUID();
    t.after(() => rmSync(new URL('../../.tmp/starsector-proxy/' + id, import.meta.url), { recursive: true, force: true }));
    const env = { ...process.env };
    delete env.PWD; // Match the actual reproduced native environment.
    const result = spawnSync(process.execPath, ['--input-type=module', '-e', boundary, runner,
        '--opencode', 'external-cli-boundary', '--run-id', id],
        { cwd: root, env, encoding: 'utf8', timeout: 10000 });
    assert.equal(result.status, 1, 'Missing session/PNG assertions remain active in the real runner');
    assert.match(result.stderr, /No valid OpenCode session ID/);
    const contexts = [...result.stdout.matchAll(/API_CONTEXT:(.+)/g)].map(match => JSON.parse(match[1]));
    assert.equal(contexts.length, 2, 'Both registration and readiness context are checked');
    const run = result.stdout.match(/RUN_CONTEXT:(.+)/);
    assert.ok(run, 'The real model-run boundary was reached');
    const clientDirectory = JSON.parse(run[1]);
    for (const context of contexts) {
        assert.equal(context.directory, context.cliDirectory, 'API directory must match the actual OS cwd');
        assert.equal(context.directory, clientDirectory, 'Registration/readiness and model client share a location');
    }
});

test('MCP readiness mode closes the owned service without model, export or frames', t => {
    const id = 'model-test-' + randomUUID();
    const evidence = new URL('../../.tmp/starsector-proxy/' + id + '/', import.meta.url);
    t.after(() => rmSync(evidence, { recursive: true, force: true }));
    const result = spawnSync(process.execPath, ['--input-type=module', '-e', boundary, runner,
        '--opencode', 'external-cli-boundary', '--run-id', id, '--check-mcp-only'],
        { cwd: root, encoding: 'utf8', timeout: 10000 });
    assert.equal(result.status, 0, result.stderr);
    assert.deepEqual([...result.stdout.matchAll(/EXTERNAL_COMMAND:(\w+)/g)].map(match => match[1]), ['serve', 'api', 'api']);
    assert.doesNotMatch(result.stdout, /REQUESTED_MODEL:/);
    const report = JSON.parse(readFileSync(new URL('mcp-readiness.json', evidence), 'utf8'));
    assert.equal(report.status, 'MCP_READY');
    const contexts = [...result.stdout.matchAll(/API_CONTEXT:(.+)/g)].map(match => JSON.parse(match[1]));
    assert.equal(contexts.length, 2);
    for (const context of contexts) assert.equal(report.directory, context.directory);
    assert.equal(report.modelDispatched, false);
    assert.equal(report.agentDispatched, false);
    for (const name of ['events.jsonl', 'session.jsonl', 'frames.json']) assert.equal(existsSync(new URL(name, evidence)), false);
});

test('MCP readiness refuses historical export before external or evidence effects', () => {
    const id = 'model-test-' + randomUUID();
    const evidence = new URL('../../.tmp/starsector-proxy/' + id, import.meta.url);
    const result = spawnSync(process.execPath, ['--input-type=module', '-e', boundary, runner,
        '--opencode', 'external-cli-boundary', '--run-id', id, '--check-mcp-only', '--session-export', 'ses_historical'],
        { cwd: root, encoding: 'utf8', timeout: 10000 });
    assert.equal(result.status, 1);
    assert.match(result.stderr, /MCP readiness cannot export a historical session/);
    assert.doesNotMatch(result.stdout, /EXTERNAL_COMMAND:/);
    assert.equal(existsSync(evidence), false);
});

test('invalid explicit model is rejected before CLI invocation or evidence creation', () => {
    const id = 'model-test-' + randomUUID();
    const evidence = new URL('../../.tmp/starsector-proxy/' + id, import.meta.url);
    try {
        const result = spawnSync(process.execPath, ['--input-type=module', '-e', boundary, runner,
            '--opencode', 'external-cli-boundary', '--run-id', id, '--model', ''],
            { cwd: root, encoding: 'utf8', timeout: 10000 });
        assert.equal(result.error, undefined);
        assert.equal(result.status, 1);
        assert.doesNotMatch(result.stdout, /REQUESTED_MODEL:/);
        assert.match(result.stderr, /Provide a provider\/model identifier/);
        assert.equal(existsSync(evidence), false);
    } finally { rmSync(evidence, { recursive: true, force: true }); }
});

test('an input agent without an explicit current batch binding refuses before client or evidence effects', () => {
    const id = 'model-test-' + randomUUID();
    const evidence = new URL('../../.tmp/starsector-proxy/' + id, import.meta.url);
    try {
        const result = spawnSync(process.execPath, ['--input-type=module', '-e', boundary, runner,
            '--opencode', 'external-cli-boundary', '--run-id', id, '--agent', 'starsector-proxy', '--wait-for-report'],
            { cwd: root, encoding: 'utf8', timeout: 10000 });
        assert.equal(result.status, 1);
        assert.match(result.stderr, /Input agent requires an explicit current batch authorization/);
        assert.doesNotMatch(result.stdout, /EXTERNAL_COMMAND:/);
        assert.equal(existsSync(evidence), false);
    } finally { rmSync(evidence, { recursive: true, force: true }); }
});

test('current guarded input binding reaches MCP registration and cannot be reused by another invocation', t => {
    const f = fixture(); t.after(() => f.close());
    const first = 'model-test-' + randomUUID(), second = 'model-test-' + randomUUID();
    for (const id of [first, second]) t.after(() => rmSync(new URL('../../.tmp/starsector-proxy/' + id, import.meta.url), { recursive: true, force: true }));
    const invoke = /** @param {string} id */ id => spawnSync(process.execPath, ['--input-type=module', '-e', boundary, runner,
        '--opencode', 'external-cli-boundary', '--run-id', id, '--agent', 'starsector-proxy', '--wait-for-report',
        '--batch-authorization', f.path], { cwd: root, encoding: 'utf8', timeout: 10000 });
    const result = invoke(first);
    assert.equal(result.status, 1); assert.match(result.stderr, /No valid OpenCode session ID/);
    assert.match(result.stdout, /REQUESTED_AGENT:starsector-proxy/);
    const registration = result.stdout.match(/REGISTERED_BATCH:(.+)/); assert.ok(registration);
    const registered = JSON.parse(registration[1]);
    assert.match(registered.command[1], /guarded-mcp\.mjs$/); assert.equal(registered.authorization, f.path);
    assert.equal(registered.runID, first);
    const reused = invoke(second);
    assert.equal(reused.status, 1); assert.match(reused.stderr, /EEXIST/);
    assert.doesNotMatch(reused.stdout, /EXTERNAL_COMMAND:/);
});

for (const args of [[], ['--session-export', 'ses_historical'], ['--check-mcp-only']]) {
    test('batch binding cannot enable an observer, historical export or readiness call: ' + JSON.stringify(args), () => {
        const id = 'model-test-' + randomUUID();
        const result = spawnSync(process.execPath, ['--input-type=module', '-e', boundary, runner,
            '--opencode', 'external-cli-boundary', '--run-id', id, '--batch-authorization', 'unread-path', ...args],
            { cwd: root, encoding: 'utf8', timeout: 10000 });
        assert.equal(result.status, 1); assert.match(result.stderr, /Batch authorization requires a new input-agent run/);
        assert.doesNotMatch(result.stdout, /EXTERNAL_COMMAND:/);
        assert.equal(existsSync(new URL('../../.tmp/starsector-proxy/' + id, import.meta.url)), false);
    });
}

for (const args of [[], ['--model', 'unsloth-studio/unsloth/Qwen3.8-27B-GGUF:UD-Q4_K_XL', '--session-export', 'ses_historical']]) {
    test('local inference control refuses an unsupported model or historical export before effects: ' + JSON.stringify(args), () => {
        const id = 'model-test-' + randomUUID();
        const evidence = new URL('../../.tmp/starsector-proxy/' + id, import.meta.url);
        const result = spawnSync(process.execPath, ['--input-type=module', '-e', boundary, runner,
            '--opencode', 'external-cli-boundary', '--run-id', id, '--disable-thinking', ...args],
            { cwd: root, encoding: 'utf8', timeout: 10000 });
        assert.equal(result.status, 1);
        assert.match(result.stderr, /Inference control requires a new run of the qualified local model/);
        assert.doesNotMatch(result.stdout, /REQUESTED_/);
        assert.equal(existsSync(evidence), false);
    });
}
