// Real-account diagnostic gate: exit nonzero unless the requested provider has quota.
import {execFile} from 'node:child_process';
import {promisify} from 'node:util';
import {fileURLToPath} from 'node:url';
import {join} from 'node:path';
const runtime=fileURLToPath(new URL('../desktop-p0/target/release/quota-helper/',import.meta.url));
const workdir=process.argv.includes('--smoke-workdir')
  ? fileURLToPath(new URL('../.scratch/quota-smoke-runtime/',import.meta.url))
  : join(process.env.LOCALAPPDATA,'com.agentmeter.p0/quota-workspace');
try {
  const {stdout}=await promisify(execFile)(join(runtime,'node.exe'),[join(runtime,'quota-desktop.mjs'),'claude',workdir,'--allow-workspace-trust'],
    {windowsHide:true,timeout:40000,maxBuffer:262144});
  const result=JSON.parse(stdout).results[0];
  console.log(JSON.stringify(result));
  process.exitCode=result.status==='PASS' && result.quota?.length===2 ? 0:1;
} catch { console.log('packaged_probe_failed');process.exitCode=1; }
