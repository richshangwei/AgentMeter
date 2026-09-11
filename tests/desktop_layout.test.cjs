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

test('page capacity keeps the count topology until a card would become unreadable', () => {
  assert.equal(context.desktopPageCapacity(1026,528,4,12),4,'default 1080 × 640 window keeps 2 × 2');
  assert.equal(context.desktopPageCapacity(1350,424,4,12),4,'short wide windows keep 2 × 2 with wide cards');
  assert.equal(context.desktopPageCapacity(616,424,4,12),2,'minimum window shows two readable cards per page');
  assert.equal(context.desktopPageCapacity(1026,528,3,12),4,'three cards keep the 1 × 3 composition');
  assert.equal(context.desktopPageCapacity(NaN,NaN,4,12),4,'unmeasured layouts keep the default');
  assert.equal(context.desktopCardShape(1026,168),'wide');
  assert.equal(context.desktopCardShape(507,258),'normal');
});

test('quota tiles always fit the measured region and pick a readable variant', () => {
  for (const [width,height] of [[483,140],[1200,540],[278,92],[306,100],[280,300],[616,300],[150,60],[900,120]]) {
    for (let count = 1; count <= 12; count++) {
      const layout = context.quotaTileLayout(width,height,count);
      assert.ok(layout.columns * layout.rows >= count, `${width}x${height} n=${count} cells`);
      assert.ok(layout.columns * layout.tileWidth + (layout.columns - 1) * layout.gap <= width + .5, `${width}x${height} n=${count} width`);
      assert.ok(layout.rows * layout.tileHeight + (layout.rows - 1) * layout.gap <= height + .5, `${width}x${height} n=${count} height`);
      if (layout.variant === 'ring' || layout.variant === 'stack') {
        assert.ok(layout.ring <= layout.tileHeight - layout.pad * 2 + .5, `${width}x${height} n=${count} ring height`);
        assert.ok(layout.ring <= layout.tileWidth, `${width}x${height} n=${count} ring width`);
      }
    }
  }
  assert.equal(context.quotaTileLayout(483,140,8).columns,4,'eight windows use two readable rows, not eight thin rows');
  assert.equal(context.quotaTileLayout(0,140,8),null);
});

test('Codex quota labels use the actual period and understandable group names', () => {
  assert.equal(context.quotaLabel({limit_id:'codex',window:'primary',window_duration_mins:10080}),'常規使用額度 · 每週使用上限');
  assert.equal(context.quotaLabel({limit_id:'codex',window:'primary',window_duration_mins:300}),'常規使用額度 · 5 小時用量限制');
  assert.equal(context.quotaLabel({limit_id:'codex_bengalfox',limit_name:'GPT-5.3-Codex-Spark',window:'secondary',window_duration_mins:10080}),'GPT-5.3-Codex-Spark · 每週使用上限');
  assert.equal(context.quotaLabel({limit_id:'base_model_inference',limit_name:'gpt-reserve',window:'primary',window_duration_mins:10080}),'備用模型額度 · 每週使用上限');
  assert.doesNotMatch(context.quotaLabel({limit_id:'future',window:'primary'}),/主視窗|次視窗|primary|secondary/);
});
