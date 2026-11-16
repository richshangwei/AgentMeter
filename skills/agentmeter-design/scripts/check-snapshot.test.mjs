// Run: node --test skills/agentmeter-design/scripts
// Each test mutates the known-good example and asserts exactly which invariant fires.
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { checkSnapshot } from './check-snapshot.mjs';

const examplePath = fileURLToPath(new URL('../assets/dashboard-snapshot.example.json', import.meta.url));
const scriptPath = fileURLToPath(new URL('./check-snapshot.mjs', import.meta.url));
const good = () => JSON.parse(readFileSync(examplePath, 'utf8'));
const provider = (snapshot, id) => snapshot.providers.find(p => p.provider === id);
const errorsOf = (snapshot, options) => checkSnapshot(snapshot, options).errors;
const mentions = (list, text) => list.some(line => line.includes(text));

test('the shipped example passes with no errors', () => {
  const { errors } = checkSnapshot(good());
  assert.deepEqual(errors, []);
});

test('a partial snapshot is rejected: all four providers must be present', () => {
  const snapshot = good();
  snapshot.providers = snapshot.providers.filter(p => p.provider !== 'copilot');
  assert.ok(mentions(errorsOf(snapshot), 'providers.copilot: missing'));
});

test('unknown is null, never a string placeholder or an out-of-range number', () => {
  const snapshot = good();
  provider(snapshot, 'codex').quota_windows[0].remaining_percent = '65';
  assert.ok(mentions(errorsOf(snapshot), 'finite number or null'));
  provider(snapshot, 'codex').quota_windows[0].remaining_percent = 101;
  assert.ok(mentions(errorsOf(snapshot), 'outside 0..100'));
});

test('used and remaining must agree; an over-limit window displays 0 remaining', () => {
  const snapshot = good();
  provider(snapshot, 'codex').quota_windows[0].remaining_percent = 50; // used 35 -> should be 65
  assert.ok(mentions(errorsOf(snapshot), 'disagree'));

  const over = good();
  Object.assign(provider(over, 'codex').quota_windows[0], { used_percent: 120, remaining_percent: 10 });
  assert.ok(mentions(errorsOf(over), 'over limit'));
  Object.assign(provider(over, 'codex').quota_windows[0], { used_percent: 120, remaining_percent: 0 });
  assert.deepEqual(errorsOf(over), []);
});

test('axes are validated independently and failure_code requires an error-like state', () => {
  const snapshot = good();
  provider(snapshot, 'codex').collection_state = 'broken';
  assert.ok(mentions(errorsOf(snapshot), 'collection_state'));

  const failing = good();
  provider(failing, 'codex').failure_code = 'timeout'; // state is still "ready"
  assert.ok(mentions(errorsOf(failing), 'requires collection_state error|backing_off'));
});

test('official is not stable: data_quality never accepts a maturity value', () => {
  const snapshot = good();
  provider(snapshot, 'codex').data_quality = 'stable';
  assert.ok(mentions(errorsOf(snapshot), 'data_quality'));
});

test('freshness needs a timestamp, and a failed refresh may not stay "fresh"', () => {
  const snapshot = good();
  provider(snapshot, 'antigravity').freshness = 'stale'; // never collected
  assert.ok(mentions(errorsOf(snapshot), 'requires collected_at'));

  const failed = good();
  const claude = provider(failed, 'claude');
  claude.freshness = 'fresh'; // still collection_state "error"
  assert.ok(mentions(errorsOf(failed), 'contradicts collection_state'));
});

test('Copilot reset time must stay null (it is not trustworthy)', () => {
  const snapshot = good();
  provider(snapshot, 'copilot').quota_windows[0].resets_at = '2026-10-01T00:00:00Z';
  assert.ok(mentions(errorsOf(snapshot), 'Copilot reset time'));
});

test('documented transients are tolerated only in their narrow case', () => {
  const blank = good();
  Object.assign(provider(blank, 'antigravity'), { availability: 'unknown', data_quality: 'unknown' });
  const result = checkSnapshot(blank);
  assert.deepEqual(result.errors, []);
  assert.ok(result.warnings.length >= 2);

  // "unknown" availability is NOT acceptable once the provider carries an observation.
  const observed = good();
  provider(observed, 'codex').availability = 'unknown';
  assert.ok(mentions(errorsOf(observed), 'availability'));
});

test('privacy: credentials, absolute paths, bearer tokens and raw e-mails never appear', () => {
  const withToken = good();
  provider(withToken, 'codex').token = 'PRIVATE';
  assert.ok(mentions(errorsOf(withToken), 'forbidden key'));

  const withPath = good();
  provider(withPath, 'codex').quota_windows[0].label = 'C:\\Users\\someone\\.codex\\auth.json';
  assert.ok(mentions(errorsOf(withPath), 'absolute Windows path'));

  const withBearer = good();
  provider(withBearer, 'codex').note = 'Authorization: Bearer abcdef1234567890';
  assert.ok(mentions(errorsOf(withBearer), 'bearer'));

  const email = good();
  provider(email, 'codex').provider_account = 'someone@example.com';
  assert.ok(mentions(errorsOf(email), 'unmasked e-mail'));
  provider(email, 'codex').provider_account = 's***@example.com';
  assert.deepEqual(errorsOf(email), []);
});

test('paused snapshots cannot claim active collection', () => {
  const snapshot = good();
  snapshot.paused = true;
  provider(snapshot, 'codex').collection_state = 'collecting';
  assert.ok(mentions(errorsOf(snapshot), 'paused snapshots'));
});

test('revision rules: monotonic in a stream, bump only on change, free across streams', () => {
  const before = good();

  const backwards = good();
  backwards.revision = 41;
  assert.ok(mentions(errorsOf(backwards, { previous: before }), 'went backwards'));

  const silentChange = good();
  provider(silentChange, 'codex').quota_windows[0].remaining_percent = 64;
  provider(silentChange, 'codex').quota_windows[0].used_percent = 36;
  assert.ok(mentions(errorsOf(silentChange, { previous: before }), 'revision did not advance'));

  const pointlessBump = good();
  pointlessBump.revision = 43;
  const bump = checkSnapshot(pointlessBump, { previous: before });
  assert.deepEqual(bump.errors, []);
  assert.ok(mentions(bump.warnings, 'unchanged publishes'));

  const restarted = good();
  restarted.stream_id = 'example-run-0002';
  restarted.revision = 1;
  const restart = checkSnapshot(restarted, { previous: before });
  assert.deepEqual(restart.errors, []);
  assert.ok(mentions(restart.warnings, 'new stream'));
});

test('CLI exit codes: 0 for the example, 1 for a violation, 2 for unreadable input', () => {
  const run = args => spawnSync(process.execPath, [scriptPath, ...args], { encoding: 'utf8' });
  assert.equal(run([examplePath]).status, 0);
  assert.equal(run([]).status, 2);
  assert.equal(run([examplePath, '--providers', 'codex,claude,copilot,antigravity,cursor']).status, 1);
  assert.equal(run(['does-not-exist.json']).status, 2);
});
