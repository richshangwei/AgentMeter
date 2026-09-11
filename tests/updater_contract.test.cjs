const {test} = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');

const source = fs.readFileSync(path.join(__dirname,'../desktop-p0/src/updater.rs'),'utf8');

test('monitoring stops only after a verified update is ready to launch', () => {
  assert.match(source,/\.on_before_exit\(move \|\|/);
  const installCommand = source.slice(source.indexOf('pub async fn install_update'));
  assert.doesNotMatch(installCommand,/\.stop_and_wait\(\)/);
  assert.doesNotMatch(installCommand,/\.download\(/);
  assert.match(installCommand,/\.install\(bytes\.as_slice\(\)\)/);
  assert.match(source.slice(source.indexOf('pub async fn download_update'), source.indexOf('pub async fn install_update')), /\.download\(/);
});

test('transient download or extraction failures retain the checked update for retry', () => {
  const installCommand = source.slice(source.indexOf('pub async fn install_update'));
  assert.match(installCommand,/data\.bytes\.clone\(\)/);
  assert.doesNotMatch(installCommand,/\.take\(\)/);
  assert.match(source,/failure_retains_verified_bytes/);
});
