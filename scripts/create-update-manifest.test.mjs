import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { createManifest } from './create-update-manifest.mjs';

function fixture(fn) {
  const directory = mkdtempSync(join(tmpdir(), 'agentmeter-manifest-test-'));
  const installer = join(directory, 'AgentMeter P0_0.2.0_x64-setup.exe');
  const signature = `${installer}.sig`;
  // Shape-only synthetic fixture, not a cryptographic signature or signing key.
  const encoded = Buffer.from(`untrusted comment: test\n${Buffer.alloc(74).toString('base64')}\ntrusted comment: test\n${Buffer.alloc(64).toString('base64')}\n`).toString('base64');
  writeFileSync(installer, 'fixture');
  writeFileSync(signature, encoded);
  try { fn({ version: '0.2.0', installer, signature, now: new Date('2026-09-11T00:00:00Z') }); }
  finally { rmSync(directory, { recursive: true }); }
}

test('creates static Tauri Windows x64 manifest with exact signature and pinned release URL', () => fixture(input => {
  const manifest = createManifest(input);
  assert.equal(manifest.version, '0.2.0');
  assert.equal(manifest.pub_date, '2026-09-11T00:00:00.000Z');
  assert.equal(manifest.platforms['windows-x86_64'].url, 'https://github.com/richshangwei/AgentMeter/releases/download/v0.2.0/AgentMeter%20P0_0.2.0_x64-setup.exe');
  assert.equal(Object.keys(manifest.platforms).length, 1);
}));
test('rejects malformed and nonstable versions', () => fixture(input => {
  for (const version of ['', 'v0.2.0', '01.2.0', '0.2', '0.2.0-beta', '../0.2.0']) assert.throws(() => createManifest({ ...input, version }), /Version/);
}));
test('rejects missing, empty or wrong-version installers', () => fixture(input => {
  assert.throws(() => createManifest({ ...input, version: '0.3.0' }), /installer/);
  writeFileSync(input.installer, '');
  assert.throws(() => createManifest(input), /installer/);
  rmSync(input.installer);
  assert.throws(() => createManifest(input));
}));
test('rejects missing, unrelated, empty and malformed signatures', () => fixture(input => {
  assert.throws(() => createManifest({ ...input, signature: input.installer }), /sibling/);
  for (const value of ['', 'not base64', Buffer.from('not minisign').toString('base64')]) {
    writeFileSync(input.signature, value);
    assert.throws(() => createManifest(input), /signature/);
  }
  rmSync(input.signature);
  assert.throws(() => createManifest(input));
}));
test('rejects untrusted or mismatched release URLs', () => fixture(input => {
  for (const url of ['http://github.com/richshangwei/AgentMeter/releases/download/v0.2.0/a.exe', 'https://evil.example/a.exe', 'https://github.com/other/AgentMeter/releases/download/v0.2.0/a.exe', 'https://github.com/richshangwei/AgentMeter/releases/download/v0.3.0/a.exe']) {
    assert.throws(() => createManifest({ ...input, url }), /URL/);
  }
}));
test('checks supplied public key structure and signing key ID', () => fixture(input => {
  const encodeKey = bytes => Buffer.from(`untrusted comment: test\n${bytes.toString('base64')}\n`).toString('base64');
  assert.doesNotThrow(() => createManifest({ ...input, publicKey: encodeKey(Buffer.alloc(42)) }));
  assert.throws(() => createManifest({ ...input, publicKey: 'invalid' }), /public key/);
  const mismatch = Buffer.alloc(42);
  mismatch[2] = 1;
  assert.throws(() => createManifest({ ...input, publicKey: encodeKey(mismatch) }), /key ID/);
}));
