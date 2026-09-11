const {test} = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');

const source = fs.readFileSync(path.join(__dirname,'../desktop-p0/src/updater.rs'),'utf8');

test('monitoring stops only after a verified update is ready to launch', () => {
  assert.match(source,/\.on_before_exit\(move \|\|/);
  const installCommand = source.slice(source.indexOf('pub async fn install_update'));
  assert.doesNotMatch(installCommand,/\.stop_and_wait\(\)/);
  assert.match(installCommand,/update\.download/);
  assert.match(installCommand,/update\.install\(bytes\)/);
});

test('transient download or extraction failures retain the checked update for retry', () => {
  const installCommand = source.slice(source.indexOf('pub async fn install_update'));
  assert.ok((installCommand.match(/\*pending = Some\(update\);/g) || []).length >= 2);
});
