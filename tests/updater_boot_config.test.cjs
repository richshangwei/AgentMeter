const {test} = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');

test('packaged startup gives the updater plugin a deserializable base config', () => {
  const config = JSON.parse(fs.readFileSync(path.join(__dirname,'../desktop-p0/tauri.conf.json'),'utf8'));
  assert.equal(typeof config.plugins?.updater,'object');
  assert.notEqual(config.plugins.updater,null);
  assert.equal(typeof config.plugins.updater.pubkey,'string');
});
