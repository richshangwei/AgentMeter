const {test} = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');

test('the bundled ConPTY cleanup helper is always forked without a visible Windows console', () => {
  for (const root of ['scripts/quota-smoke-support','desktop-p0/resources/quota-helper/quota-smoke-support']) {
    const source = fs.readFileSync(path.join(__dirname,'..',root,'node_modules/node-pty/lib/windowsPtyAgent.js'),'utf8');
    assert.match(source,/child_process_1\.fork\([^;]+\{\s*windowsHide:\s*true\s*\}\)/s, root);
  }
});

test('runtime preparation reapplies the pinned node-pty no-window patch after npm ci', () => {
  const source = fs.readFileSync(path.join(__dirname,'../scripts/prepare-desktop-quota.ps1'),'utf8');
  assert.match(source,/npm\.cmd/);
  assert.match(source,/ci --ignore-scripts --prefix/);
  assert.match(source,/windowsHide:\s*true/);
  assert.match(source,/conpty_console_list_agent/);
});
