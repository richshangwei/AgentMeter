const {test} = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const ui = name => fs.readFileSync(path.join(__dirname,'../desktop-p0/ui',name),'utf8');
const p = vm.createContext({});
vm.runInContext(ui('pets.js'),p);

test('each core provider has its own animal and future providers get a stable one', () => {
  const kinds = ['codex','claude','copilot','antigravity'].map(id=>p.petKindFor(id));
  assert.deepEqual(kinds,['cat','dog','rabbit','hamster']);
  for (const id of ['future-agent','constructor','toString']) {
    assert.equal(p.petKindFor(id),p.petKindFor(id));
    assert.ok(vm.runInContext('Object.keys(petKinds)',p).includes(p.petKindFor(id)));
  }
});

test('faster consumption means a shorter stride within readable bounds', () => {
  assert.equal(p.petStrideSeconds(0),0);
  assert.equal(p.petStrideSeconds(NaN),0);
  const strides = [.5,2,10,40,1000].map(rate=>p.petStrideSeconds(rate));
  for (let i=1;i<strides.length;i++) assert.ok(strides[i]<=strides[i-1]);
  assert.ok(strides[0]<=.62&&strides.at(-1)===.16);
});

test('pets run only while quota is consumed, rest otherwise and tire when it is used up', () => {
  assert.equal(p.petMood({state:'running'},12),'run');
  assert.equal(p.petMood({state:'idle'},0),'sleep');
  assert.equal(p.petMood(null,0),'sleep');
  assert.equal(p.petMood({state:'exhausted'},0),'tired');
});

test('the pets theme is selectable, recognised and loaded by both windows', () => {
  assert.match(ui('index.html'),/<option value="pets">可愛動物<\/option>/);
  assert.match(ui('theme.js'),/'pets', 'hamster'\]/);
  for (const page of ['index.html','hud.html']) for (const asset of ['pets.css','theme.js','forecast.js','pets.js','hamsters.css','hamsters.js']) assert.ok(ui(page).includes(`"${asset}"`),`${page} loads ${asset}`);
  // CSP allows same-origin assets only: no data: URIs, inline style or HTML injection.
  for (const file of ['pets.js','hamsters.js','lanes.js','hud.js']) assert.doesNotMatch(ui(file),/innerHTML|data:|setAttribute\('style'/);
});
