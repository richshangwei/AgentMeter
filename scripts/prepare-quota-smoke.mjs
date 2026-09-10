// Pinned official binaries only. No installer scripts, PATH edits, or global updates.
import { mkdir, writeFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { fileURLToPath } from 'node:url';
const dir = fileURLToPath(new URL('../.scratch/quota-smoke-runtime/',import.meta.url));
await mkdir(dir,{recursive:true});
const url = 'https://storage.googleapis.com/antigravity-public/antigravity-cli/1.1.28-5576113066475520/windows-x64/cli_windows_x64.exe';
const expected = 'df10becfbc71ef23786c2ce9a70c5dbd8008d707019d878a69c4f0e5b58d95aa9b1ac6ad0ef5763205d7eaf1a85050d5429e601b662a9fc2d9022cdfc35e98b9';
const response = await fetch(url,{signal:AbortSignal.timeout(60_000),redirect:'error'});
if (!response.ok) throw new Error(`Download HTTP ${response.status}`);
const data = Buffer.from(await response.arrayBuffer());
if (createHash('sha512').update(data).digest('hex') !== expected) throw new Error('SHA512 mismatch; refusing executable');
await writeFile(dir+'agy.exe',data,{flag:'wx'});
console.log('Verified Antigravity CLI 1.1.28 staged locally; no global installation.');
