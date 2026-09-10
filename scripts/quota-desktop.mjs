// Bundled first-party collector entry. No files, Billing, or arbitrary commands from UI.
import {collectProviders} from './quota-smoke.mjs';
import {dirname,isAbsolute} from 'node:path';
import {fileURLToPath} from 'node:url';
const [provider,workdir,trust] = process.argv.slice(2);
if (!['all','codex','claude','copilot','antigravity'].includes(provider) || !workdir || !isAbsolute(workdir) ||
    (trust !== undefined && trust !== '--allow-workspace-trust')) process.exit(2);
try {
  const results = await collectProviders({provider,workdir,runtimeDir:dirname(fileURLToPath(import.meta.url)),allowWorkspaceTrust:trust === '--allow-workspace-trust'});
  console.log(JSON.stringify({schema:'agentmeter.live-quota/v1',results}));
} catch {
  console.log(JSON.stringify({schema:'agentmeter.live-quota/v1',error:'collector_failed'}));
  process.exitCode = 1;
}
