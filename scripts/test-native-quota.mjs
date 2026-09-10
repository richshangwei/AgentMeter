// Exercises the release executable, including its bounded Windows process lifecycle.
import {execFile} from 'node:child_process';
import {promisify} from 'node:util';
import {readFileSync,writeFileSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {fileURLToPath} from 'node:url';
const executable=fileURLToPath(new URL('../desktop-p0/target/release/agentmeter-desktop-p0.exe',import.meta.url));
try {
  const {stdout}=await promisify(execFile)(executable,['--quota-collect'],{windowsHide:true,timeout:180000,maxBuffer:262144});
  const report=JSON.parse(stdout);
  const providers=['codex','claude','copilot','antigravity'];
  const pass=report.schema==='agentmeter.live-quota/v1' && providers.every(provider=>{
    const results=report.results?.filter(r=>r.provider===provider) || [];
    return results.length===1 && results[0].status==='PASS' && results[0].quota?.length>0;
  });
  const evidence={schema:'agentmeter.native-quota-evidence/v1',checked_at:new Date().toISOString(),
    outcome:pass?'PASS':'FAIL',executable_sha256:createHash('sha256').update(readFileSync(executable)).digest('hex'),report};
  const file=fileURLToPath(new URL('../docs/evidence/native-quota-'+Date.now()+'.json',import.meta.url));
  writeFileSync(file,JSON.stringify(evidence,null,2)+'\n',{flag:'wx'});
  console.log(JSON.stringify(evidence,null,2));
  console.log('Evidence: '+file);
  process.exitCode=pass?0:1;
} catch { console.log('native_quota_probe_failed');process.exitCode=1; }
