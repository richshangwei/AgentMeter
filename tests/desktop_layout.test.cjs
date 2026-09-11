const {test} = require('node:test');
const assert = require('node:assert/strict');
const vm = require('node:vm');
const fs = require('node:fs');
const path = require('node:path');

const context = vm.createContext({});
vm.runInContext(fs.readFileSync(path.join(__dirname,'../desktop-p0/ui/layout.js'),'utf8'),context);

test('dashboard has no add tile and keeps settings as the monitor-management entry point',()=>{
  const html=fs.readFileSync(path.join(__dirname,'../desktop-p0/ui/index.html'),'utf8');
  assert.doesNotMatch(html,/desktop-add-card|desktop-add-monitor/);
  assert.match(html,/id="app-settings"/);
  assert.match(html,/id="desktop-monitor-options"/);
});

test('desktop card geometry follows the exact 1/2/3/4 full-screen composition', () => {
  const matrix = [
    [1,{columns:1,rows:1,pageSize:4}],
    [2,{columns:2,rows:1,pageSize:4}],
    [3,{columns:1,rows:3,pageSize:4}],
    [4,{columns:2,rows:2,pageSize:4}],
  ];
  for (const [width,height] of [[1452,1086],[1080,640],[640,520],[390,844]]) {
    for (const [count,expected] of matrix) {
      assert.deepEqual({...context.desktopViewportGrid(width,height,count)},expected,`${width}x${height} count=${count}`);
    }
  }
});

test('desktop monitor pagination clamps pages and keeps future providers', () => {
  const available = Array.from({length:12},(_,index)=>`provider-${index+1}`);
  assert.deepEqual(Array.from(context.normalizeDesktopSelection(null,available)),available);
  const first = context.pagedMonitorIds(available,0,4);
  const last = context.pagedMonitorIds(available,99,4);
  assert.equal(first.pages,3);
  assert.deepEqual(Array.from(first.ids),available.slice(0,4));
  assert.equal(last.page,2);
  assert.deepEqual(Array.from(last.ids),available.slice(8));
  assert.deepEqual({...context.desktopViewportGrid(1080,640,last.ids.length)},{columns:2,rows:2,pageSize:4});
});

test('desktop settings can reorder selected monitors without changing membership', () => {
  const selected=['codex','claude','copilot','antigravity'];
  assert.deepEqual(Array.from(context.moveDesktopMonitor(selected,'codex',-1)),selected);
  assert.deepEqual(Array.from(context.moveDesktopMonitor(selected,'antigravity',1)),selected);
  assert.deepEqual(Array.from(context.moveDesktopMonitor(selected,'copilot',-1)),['codex','copilot','claude','antigravity']);
  assert.deepEqual(Array.from(context.moveDesktopMonitor(selected,'claude',1)),['codex','copilot','claude','antigravity']);
});
