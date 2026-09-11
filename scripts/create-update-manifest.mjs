import { readFileSync, statSync, writeFileSync } from 'node:fs';
import { basename, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const releaseRoot = 'https://github.com/richshangwei/AgentMeter/releases/download/';
const stableVersion = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/;
const base64 = /^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$/;

export function createManifest({ version, installer, signature, publicKey, url, notes = '', now = new Date() }) {
  if (!stableVersion.test(version)) throw new Error('Version must be an explicit stable x.y.z version.');
  const expectedName = `AgentMeter-P0_${version}_x64-setup.exe`;
  if (!installer || basename(installer) !== expectedName || !statSync(installer).isFile() || statSync(installer).size === 0) {
    throw new Error('Expected a nonempty AgentMeter x64 NSIS installer matching the version.');
  }
  if (resolve(signature || '') !== resolve(`${installer}.sig`)) throw new Error('Signature must be the installer .exe.sig sibling.');
  const encoded = readFileSync(signature, 'utf8').trim();
  if (!encoded || !base64.test(encoded)) throw new Error('Malformed updater signature encoding.');
  const decoded = Buffer.from(encoded, 'base64').toString('utf8').trimEnd().split(/\r?\n/);
  if (decoded.length !== 4 || !decoded[0].startsWith('untrusted comment: ') || !decoded[2].startsWith('trusted comment: ') ||
      !base64.test(decoded[1]) || Buffer.from(decoded[1], 'base64').length !== 74 ||
      !base64.test(decoded[3]) || Buffer.from(decoded[3], 'base64').length !== 64) {
    throw new Error('Malformed Tauri minisign signature structure.');
  }
  if (publicKey !== undefined) {
    const keyText = Buffer.from(publicKey, 'base64').toString('utf8').trimEnd().split(/\r?\n/);
    const keyBytes = Buffer.from(keyText[1] || '', 'base64');
    const signatureBytes = Buffer.from(decoded[1], 'base64');
    if (!base64.test(publicKey) || keyText.length !== 2 || !keyText[0].startsWith('untrusted comment: ') || keyBytes.length !== 42) {
      throw new Error('Malformed updater public key.');
    }
    if (!keyBytes.subarray(2, 10).equals(signatureBytes.subarray(2, 10))) throw new Error('Signature key ID does not match the application public key.');
  }
  const expectedUrl = `${releaseRoot}v${version}/${encodeURIComponent(expectedName)}`;
  if (url !== undefined && url !== expectedUrl) throw new Error('URL must match the pinned repository, version tag and installer filename.');
  return { version, notes, pub_date: now.toISOString(), platforms: { 'windows-x86_64': { signature: encoded, url: expectedUrl } } };
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    const [version, installer, output] = process.argv.slice(2);
    if (!output) throw new Error('Usage: node scripts/create-update-manifest.mjs VERSION INSTALLER OUTPUT');
    const manifest = createManifest({ version, installer, signature: `${installer}.sig`, publicKey: process.env.AGENTMETER_UPDATE_PUBLIC_KEY });
    // Never replace a previous release draft or overwrite an input artifact.
    writeFileSync(output, `${JSON.stringify(manifest, null, 2)}\n`, { flag: 'wx' });
    console.log(`Local update manifest created: ${resolve(output)}`);
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}
