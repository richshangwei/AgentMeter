// Diagnostic-only minimisation. Invalid provider exits before any account or network access.
const {spawnSync} = require('node:child_process');
const {resolve} = require('node:path');
const base = resolve('desktop-p0/target/release/quota-helper');
for (const scenario of [
  {name:'plain_entry', entry:base+'\\quota-desktop.mjs', cwd:base},
  {name:'verbatim_entry', entry:'\\\\?\\'+base+'\\quota-desktop.mjs', cwd:base},
  {name:'relative_entry_verbatim_cwd', entry:'quota-desktop.mjs', cwd:'\\\\?\\'+base},
]) {
  const result = spawnSync(base+'\\node.exe', [scenario.entry,'invalid'], {
    cwd:scenario.cwd,windowsHide:true,encoding:'utf8',timeout:5000,
  });
  console.log(JSON.stringify({scenario:scenario.name,exit:result.status,stdoutBytes:result.stdout?.length,
    errorCode:result.stderr?.match(/ERR_[A-Z_]+|MODULE_NOT_FOUND|EISDIR/)?.[0] || null}));
  if (result.status !== 2) process.exitCode = 1;
}
