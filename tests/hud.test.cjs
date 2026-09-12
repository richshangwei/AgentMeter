const {test} = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');

const context = vm.createContext({});
vm.runInContext(fs.readFileSync(path.join(__dirname,'../desktop-p0/ui/hud-model.js'),'utf8'),context);

test('HUD opacity is readable, bounded, and restored from safe defaults', () => {
  assert.equal(context.normalizeHudOpacity(undefined),72);
  assert.equal(context.normalizeHudOpacity('44'),44);
  assert.equal(context.normalizeHudOpacity(12),35);
  assert.equal(context.normalizeHudOpacity(100),95);
  assert.equal(context.normalizeHudOpacity('not-a-number'),72);
});

test('HUD selection accepts future providers but rejects reserved or unsafe ids', () => {
  const selection=context.normalizeHudSelection(['codex','future-agent','codex','desktop-cards','__proto__','bad id']);
  assert.deepEqual(Array.from(selection),['codex','future-agent']);
});

test('HUD renders selected providers in selection order with one primary value each', () => {
  const snapshot={provider_states:[
    {provider:'claude',quota_windows:[{remaining_percent:47},{remaining_percent:28}]},
    {provider:'codex',quota_windows:[{remaining_percent:82}]},
    {provider:'copilot',quota_windows:[],source_usage:[]}
  ]};
  const rows=context.hudRows(snapshot,['codex','copilot','claude'],{codex:'Codex',copilot:'GitHub Copilot',claude:'Claude Code'});
  assert.deepEqual(Array.from(rows,row=>({...row})),[
    {id:'codex',name:'Codex',value:'82%'},
    {id:'copilot',name:'GitHub Copilot',value:'—'},
    {id:'claude',name:'Claude Code',value:'47%'}
  ]);
});

test('HUD uses source usage only when quota windows are absent and preserves unknown values', () => {
  const snapshot={provider_states:[
    {provider:'copilot',quota_windows:[],source_usage:[{used:1130,entitlement:1500,unit:'interactions'}]},
    {provider:'future',quota_windows:[{remaining_percent:null}]}
  ]};
  const rows=context.hudRows(snapshot,['copilot','future'],{copilot:'GitHub Copilot'});
  assert.equal(rows[0].value,'1130 / 1500');
  assert.equal(rows[1].name,'future');
  assert.equal(rows[1].value,'—');
});

test('HUD height stays compact and bounded for the supported visible row count', () => {
  assert.equal(context.hudHeight(0),72);
  assert.equal(context.hudHeight(1),72);
  assert.equal(context.hudHeight(4),204);
  assert.equal(context.hudHeight(99),204);
});
