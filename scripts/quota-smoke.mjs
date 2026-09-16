// Read-only, real-account gate. No model turns, Billing endpoints, or raw auth output.
import { spawn, execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { createRequire } from 'node:module';
import { existsSync, readdirSync, mkdirSync, writeFileSync } from 'node:fs';
import { join, delimiter, resolve, isAbsolute } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../', import.meta.url));
const local = process.env.LOCALAPPDATA || '';
const npm = join(process.env.APPDATA || '', 'npm', 'node_modules');
const run = promisify(execFile);
const first = paths => paths.find(p => isAbsolute(p) && existsSync(p));
let workingDirectory = join(root,'.scratch/quota-smoke-runtime');
let runtimeDirectory = workingDirectory;
let allowWorkspaceTrust = false;
function findExe(name, extras = []) {
  return first([...extras, ...(process.env.PATH || '').split(delimiter).map(p => join(p, name))]);
}
function desktopCodex() {
  const dir = join(local, 'OpenAI', 'Codex', 'bin');
  try { return readdirSync(dir).map(p => join(dir, p, 'codex.exe')); } catch { return []; }
}
function rpc(exe, args, framed, env = process.env) {
  const child = spawn(exe, args, { cwd: workingDirectory, env, windowsHide: true, stdio: ['pipe','pipe','pipe'] });
  let buffer = Buffer.alloc(0), id = 0;
  const pending = new Map();
  const fail = () => { for (const p of pending.values()) p.reject(new Error('transport_failed')); };
  child.on('error', fail); child.on('exit', fail); child.stdin.on('error', fail);
  child.stderr.resume(); // Never persist provider stderr, tokens, or account identity.
  child.stdout.on('data', chunk => {
    buffer = Buffer.concat([buffer, chunk]);
    if (buffer.length > 2_000_000) { fail(); child.kill(); return; }
    while (true) {
      let end, body;
      if (framed) {
        end = buffer.indexOf('\r\n\r\n');
        if (end < 0) return;
        const match = /Content-Length: (\d+)/i.exec(buffer.subarray(0,end).toString());
        if (!match) { fail(); child.kill(); return; }
        const size = Number(match[1]);
        if (buffer.length < end + 4 + size) return;
        body = buffer.subarray(end+4,end+4+size); buffer = buffer.subarray(end+4+size);
      } else {
        end = buffer.indexOf('\n'); if (end < 0) return;
        body = buffer.subarray(0,end); buffer = buffer.subarray(end+1);
      }
      try {
        const msg = JSON.parse(body.toString());
        const p = pending.get(msg.id);
        if (p) {
          pending.delete(msg.id);
          if (msg.error) {
            const s = String(msg.error.message || '');
            const reason = /auth|login|sign.?in|401/i.test(s) ? 'authentication_required' :
              /403|forbidden|permission/i.test(s) ? 'permission_denied' :
              /method.*not|unhandled method|unsupported/i.test(s) ? 'method_unsupported' : 'provider_error';
            p.reject(new Error(reason));
          } else p.resolve(msg.result);
        }
      } catch { fail(); }
    }
  });
  function send(msg) {
    const body = JSON.stringify(msg);
    child.stdin.write(framed ? `Content-Length: ${Buffer.byteLength(body)}\r\n\r\n${body}` : body+'\n');
  }
  return {
    notify(method) { send({jsonrpc:'2.0',method,params:{}}); },
    async call(method, params = {}) {
      const requestId = ++id;
      let timer;
      try { return await new Promise((resolve,reject) => {
        pending.set(requestId,{resolve,reject});
        timer = setTimeout(() => reject(new Error('timeout')),20_000);
        send({jsonrpc:'2.0',id:requestId,method,params});
      }); } finally { clearTimeout(timer); pending.delete(requestId); }
    },
    stop() { child.stdin.end(); child.kill(); }
  };
}
export function codexWindows(result) {
  const buckets = result?.rateLimitsByLimitId ? Object.entries(result.rateLimitsByLimitId) : [['default',result?.rateLimits]];
  return buckets.flatMap(([bucket,value]) => ['primary','secondary'].flatMap(window => {
    const q = value?.[window];
    return typeof q?.usedPercent === 'number' && Number.isFinite(q.usedPercent) && q.usedPercent >= 0
      ? [{bucket,limit_id:typeof value?.limitId === 'string' ? value.limitId : bucket,
          limit_name:typeof value?.limitName === 'string' ? value.limitName : null,
          plan_type:typeof value?.planType === 'string' ? value.planType : null,
          window,window_duration_mins:typeof q.windowDurationMins === 'number' && Number.isFinite(q.windowDurationMins) ? q.windowDurationMins : null,
          remaining_percent:Math.max(0,100-q.usedPercent),resets_at:q.resetsAt ?? null}] : [];
  }));
}
export function copilotWindows(result) {
  return Object.entries(result?.quotaSnapshots || {}).flatMap(([bucket,q]) =>
    q?.entitlementRequests > 0 && typeof q?.remainingPercentage === 'number' && Number.isFinite(q.remainingPercentage) && q.remainingPercentage >= 0 && q.remainingPercentage <= 100
      ? [{bucket,remaining_percent:q.remainingPercentage,entitlement:q.entitlementRequests ?? null,used:q.usedRequests ?? null,
          reported_reset_at:q.resetDate ?? null,reset_time_verified:false}] : []);
}
export function antigravityWindows(text) {
  const rows = text.trim().split(/\r?\n/).map(line => line.split('\t'));
  if (!rows.length || rows.some(r => r.length !== 4)) return [];
  const result = [];
  for (const [bucket,window,percent,reset] of rows) {
    if (!['Gemini Models','Claude and GPT models'].includes(bucket) ||
        !['Weekly Limit Remaining','Five Hour Limit Remaining'].includes(window) ||
        !/^\d+(\.\d+)?%$/.test(percent) || !Number.isFinite(Date.parse(reset))) return [];
    const remaining = Number(percent.slice(0,-1));
    if (remaining < 0 || remaining > 100) return [];
    result.push({bucket,window,remaining_percent:remaining,resets_at:reset});
  }
  return result;
}
export function claudeWindows(screen) {
  const result = [];
  for (const [heading,window] of [['Current session','five_hour'],['Current week (all models)','seven_day']]) {
    const start = screen.indexOf(heading);
    if (start < 0) return [];
    const lines = screen.slice(start+heading.length).split('\n').filter(l => l.trim());
    const used = /(?:^|\s)(\d+(?:\.\d+)?)%\s+used\s*$/.exec(lines[0] || '');
    const reset = /^\s*Resets (.+?)\s*$/.exec(lines[1] || '');
    if (!used || Number(used[1]) > 100) return [];
    result.push({window,remaining_percent:100-Number(used[1]),reset_display:reset?.[1] ?? null});
  }
  return result;
}
export function confirmClaudeTrust(write, screen, schedule = setTimeout) {
  // First paint precedes the CLI's input handler registration on a fresh workspace.
  let stopped = false, timer;
  const yesSelected = text => /^\s*[>❯]\s*(?:\d+\.\s*)?Yes, I trust this folder\s*$/m.test(text);
  timer = schedule(() => {
    if (stopped) return;
    const text = screen();
    if (yesSelected(text)) { write('\r'); return; }
    const lines = text.split('\n');
    const no = lines.findIndex(line=>/^\s*[>❯]\s*(?:\d+\.\s*)?No, exit\s*$/.test(line));
    const yes = lines.findIndex(line=>/^\s*(?:\d+\.\s*)?Yes, I trust this folder\s*$/.test(line));
    if (no < 0 || yes < 0) return;
    write(yes > no ? '\u001b[B' : '\u001b[A');
    timer = schedule(() => { if (!stopped && yesSelected(screen())) write('\r'); },250);
  },750);
  return () => { stopped = true; clearTimeout(timer); };
}
async function claudeTerminal(exe) {
  const require = createRequire(new URL('./quota-smoke-support/package.json',import.meta.url));
  const pty = require('node-pty');
  const {Terminal} = require('@xterm/headless');
  const cwd = workingDirectory;
  mkdirSync(cwd,{recursive:true});
  const terminal = new Terminal({cols:160,rows:70,allowProposedApi:true});
  const child = pty.spawn(exe,['--safe-mode','--permission-mode','dontAsk','--tools',''],{
    name:'xterm-256color',cols:160,rows:70,cwd,
    env:{...process.env,DISABLE_AUTOUPDATER:'1'},useConpty:true
  });
  let timer, settle, trustTimer, sent = false, trustAccepted = false, exited = false;
  try {
    return await new Promise((resolve,reject) => {
      const screen = () => Array.from({length:terminal.buffer.active.length},(_,i) =>
        terminal.buffer.active.getLine(i)?.translateToString(true) || '').join('\n');
      timer = setTimeout(() => reject(new Error('terminal_quota_timeout')),25_000);
      child.onExit(() => {
        exited = true;
        reject(new Error('terminal_exited'));
      });
      child.onData(data => terminal.write(data,() => {
        const text = screen();
        if (text.includes('Yes, I trust this folder')) {
          if (!allowWorkspaceTrust) return reject(new Error('workspace_trust_required'));
          if (!trustAccepted) { trustAccepted = true; trustTimer = confirmClaudeTrust(value => child.write(value),screen); }
          return;
        }
        if (!sent && text.includes('Safe mode:') && text.includes('Claude Code')) {
          sent = true;
          child.write('/usage\r');
        }
        if (!settle && claudeWindows(text).length === 2) {
          // Allow the asynchronously fetched account data to replace initial UI values.
          settle = setTimeout(() => {
            const quota = claudeWindows(screen());
            quota.length === 2 ? resolve(quota) : reject(new Error('terminal_schema_changed'));
          },2500);
        }
      }));
    });
  } finally {
    clearTimeout(timer); clearTimeout(settle); trustTimer?.();
    if (!exited) child.kill();
    terminal.dispose();
  }
}
async function probe(provider, exe, args, framed, get, env) {
  if (!exe) return {provider,status:'BLOCKED',reason:'cli_not_found',quota:[]};
  let client;
  try {
    client = rpc(exe,args,framed,env);
    const quota = await get(client);
    return {provider,status:quota.length ? 'PASS' : 'FAIL',reason:quota.length ? 'live_quota_received' : 'quota_missing',quota,collected_at:Date.now()};
  } catch(e) { return {provider,status:'FAIL',reason:e.message,quota:[]}; }
  finally { client?.stop(); }
}
export async function collectProviders(options = {}) {
  const provider = options.provider || 'all';
  if (!['all','codex','copilot','claude','antigravity'].includes(provider)) throw new Error('unknown_provider');
  workingDirectory = options.workdir || join(root,'.scratch/quota-smoke-runtime');
  runtimeDirectory = options.runtimeDir || join(root,'.scratch/quota-smoke-runtime');
  if (!isAbsolute(workingDirectory) || !isAbsolute(runtimeDirectory)) throw new Error('absolute_runtime_required');
  allowWorkspaceTrust = options.allowWorkspaceTrust === true;
  mkdirSync(workingDirectory,{recursive:true});
  const results = [];
  if (provider === 'all' || provider === 'codex') {
  results.push(await probe('codex',findExe('codex.exe',desktopCodex()),['app-server'],false,async c => {
    await c.call('initialize',{clientInfo:{name:'agentmeter_quota_smoke',version:'0.1.0'}});
    c.notify('initialized');
    return codexWindows(await c.call('account/rateLimits/read'));
  }));
  }
  if (provider === 'all' || provider === 'copilot') {
  const copilotExe = findExe('copilot.exe',[
    join(npm,'@github/copilot/node_modules/@github/copilot-win32-x64/copilot.exe')
  ]);
  const copilotArgs = ['--headless','--stdio','--no-auto-update','--log-level','none'];
  const queryCopilot = async c => {
    await c.call('ping');
    const auth = await c.call('auth.getStatus');
    if (!auth?.isAuthenticated) throw new Error('authentication_required');
    return copilotWindows(await c.call('account.getQuota'));
  };
  let copilot = await probe('copilot',copilotExe,copilotArgs,true,queryCopilot);
  copilot.auth_source = 'copilot_cli';
  if (copilot.reason === 'authentication_required') {
    const gh = findExe('gh.exe',[join(process.env.ProgramFiles || 'C:/Program Files','GitHub CLI/gh.exe')]);
    if (gh) {
      try {
        // Credential stays in memory and is supplied only to the official runtime.
        // No credential file is inspected, no token is printed or put on argv.
        const { stdout } = await run(gh,['auth','token'],{windowsHide:true,timeout:10_000,maxBuffer:65536});
        const token = stdout.trim();
        if (token) {
          copilot = await probe('copilot',copilotExe,[...copilotArgs,'--auth-token-env','AGENTMETER_QUOTA_TOKEN'],true,queryCopilot,
            {...process.env,AGENTMETER_QUOTA_TOKEN:token});
          copilot.auth_source = 'active_github_cli_account';
        }
      } catch { /* Keep the original sanitized failure. */ }
    }
  }
  results.push(copilot);
  }
  if (provider === 'all' || provider === 'claude') {
  // Never substitute auth success or session-token usage for account quota.
  const claude = findExe('claude.exe',[join(process.env.USERPROFILE || '', '.local/bin/claude.exe'),join(npm,'@anthropic-ai/claude-code/bin/claude.exe')]);
  let claudeResult = {provider:'claude',status:'BLOCKED',reason:claude ? 'live_statusline_event_required' : 'cli_not_found',quota:[]};
  if (claude) {
    try {
      const options = {windowsHide:true,timeout:10_000,maxBuffer:65536,env:{...process.env,DISABLE_AUTOUPDATER:'1'}};
      const version = (await run(claude,['--version'],options)).stdout.match(/\d+\.\d+\.\d+/)?.[0];
      const auth = JSON.parse((await run(claude,['auth','status'],options)).stdout);
      claudeResult.version = version;
      claudeResult.authenticated = auth.loggedIn === true;
      if (!claudeResult.authenticated) claudeResult.reason = 'authentication_required';
      else {
        const quota = await claudeTerminal(claude);
        claudeResult = {...claudeResult,status:'PASS',reason:'live_terminal_quota_received',quota,
          transport:'official_cli_usage_terminal',maturity:'experimental',
          limitation:'Version-sensitive English terminal parser; reset text is not an absolute timestamp.'};
      }
    } catch(e) { claudeResult.reason = ['terminal_quota_timeout','terminal_exited','workspace_trust_required','terminal_schema_changed'].includes(e.message) ? e.message : 'terminal_probe_unavailable'; }
  }
  results.push({...claudeResult,collected_at:Date.now()});
  }
  if (provider === 'all' || provider === 'antigravity') {
  const agy = findExe('agy.exe',[join(runtimeDirectory,'agy.exe'),join(local,'agy/bin/agy.exe')]);
  let antigravity = {provider:'antigravity',status:'BLOCKED',reason:'official_cli_not_found_desktop_is_not_cli',quota:[]};
  if (agy) {
    try {
      // agy's built-in auto-updater starts a detached background process that has no
      // console of its own; its helpers then open a new, visible console (a CMD /
      // Windows Terminal flash). The CLI documents the exact boolean literal `true`;
      // `1` does not reliably disable its 15-minute updater path.
      const {stdout} = await run(agy,['--print','/usage','--print-timeout','20s'],{
        cwd:workingDirectory,windowsHide:true,timeout:25_000,maxBuffer:65536,
        env:{...process.env,AGY_CLI_DISABLE_AUTO_UPDATE:'true'}});
      const quota = antigravityWindows(stdout);
      antigravity = {provider:'antigravity',status:quota.length ? 'PASS':'FAIL',
        reason:quota.length ? 'live_cli_quota_received':'quota_schema_unrecognized',quota,
        transport:'official_cli_usage_tsv',maturity:'experimental'};
    } catch { antigravity.reason = 'cli_quota_request_failed'; antigravity.status = 'FAIL'; }
  }
  results.push({...antigravity,collected_at:Date.now()});
  }
  return results;
}
async function main() {
  const results = await collectProviders();
  const report = {schema:'agentmeter.real-quota-smoke/v1',checked_at:new Date().toISOString(),
    gate:results.every(r => r.status==='PASS') ? 'GO' : 'NO_GO',
    scope:'Live quota acquisition feasibility only, not production readiness or reset-time validation. Current local account contexts; BLOCKED does not prove provider impossibility.',results};
  const out = join(root,'docs/evidence',`quota-smoke-${Date.now()}.json`);
  mkdirSync(join(root,'docs/evidence'),{recursive:true});
  writeFileSync(out,JSON.stringify(report,null,2)+'\n',{flag:'wx'});
  console.log(JSON.stringify(report,null,2));
  console.log(`Evidence: ${out}`);
  process.exitCode = report.gate==='GO' ? 0 : 1;
}
if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) await main();
