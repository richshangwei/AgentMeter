import { readFileSync } from 'node:fs';
import { createHash, createPublicKey, verify } from 'node:crypto';
import assert from 'node:assert/strict';

// Verify the same prehashed Ed25519 minisign structure used by minisign-verify.
// Inputs are the public key and signed artifact only; no private material needed.
const [installer, publicKeyFile] = process.argv.slice(2);
if (!installer || !publicKeyFile) throw Error('Usage: node verify-update-signature.mjs INSTALLER PUBLIC_KEY_FILE');
const publicLines = Buffer.from(readFileSync(publicKeyFile, 'utf8').trim(), 'base64').toString('utf8').trim().split(/\r?\n/);
const signatureLines = Buffer.from(readFileSync(`${installer}.sig`, 'utf8').trim(), 'base64').toString('utf8').trim().split(/\r?\n/);
const pk = Buffer.from(publicLines[1], 'base64');
const sig = Buffer.from(signatureLines[1], 'base64');
assert.equal(pk.length, 42);
assert.equal(sig.length, 74);
assert.equal(sig.subarray(0, 2).toString(), 'ED');
assert(pk.subarray(2, 10).equals(sig.subarray(2, 10)));
assert(signatureLines[2].startsWith('trusted comment: '));
const key = createPublicKey({ key: Buffer.concat([Buffer.from('302a300506032b6570032100', 'hex'), pk.subarray(10)]), format: 'der', type: 'spki' });
const data = readFileSync(installer);
const digest = bytes => createHash('blake2b512').update(bytes).digest();
assert(verify(null, digest(data), key, sig.subarray(10)), 'Installer signature invalid');
assert(verify(null, Buffer.concat([sig.subarray(10), Buffer.from(signatureLines[2].slice(17))]), key, Buffer.from(signatureLines[3], 'base64')), 'Trusted comment signature invalid');
data[0] ^= 1;
assert(!verify(null, digest(data), key, sig.subarray(10)), 'Modified installer must fail verification');
console.log('PASS: installer signature, trusted comment and modified-byte rejection');
