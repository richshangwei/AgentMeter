const test = require('node:test');
const assert = require('node:assert/strict');
const vm = require('node:vm');
const fs = require('node:fs');
const path = require('node:path');
const source = fs.readFileSync(path.join(__dirname, '../ui/setup.js'), 'utf8');

function harness(overrides = {}) {
  const elements = new Map(), calls = [];
  function element(id) {
    if (!elements.has(id)) {
      const handlers = new Map();
      elements.set(id, {
        value:'', textContent:'', disabled:false, open:false, children:[],
        addEventListener(type, fn) { handlers.set(type, fn); },
        dispatchEvent(event) { return handlers.get(event.type)?.(event); },
        click() { if (!this.disabled) return this.dispatchEvent({type:'click'}); },
        replaceChildren(...children) { this.children = children; },
        showModal() { this.open = true; },
        close() { this.open = false; return this.dispatchEvent({type:'close'}); }
      });
    }
    return elements.get(id);
  }
  const replies = {
    inspect_setup: () => ({codex_installed:true, github_installed:false, claude_installed:true, claude_integration:'not_enabled', adb_path:'C:/tools/adb.exe'}),
    preview_claude_setup: ({enable}) => ({action:enable?'enable':'disable', message:'preview', settings_path:'LOCAL PRIVATE PATH', before:{command:'PRIVATE ORIGINAL'}, after:{command:'receiver'}}),
    apply_claude_setup: () => ({enabled:true, report_path:'C:/report.json', message:'enabled'}),
    ...overrides
  };
  vm.runInNewContext(source, {
    document:{getElementById:element, createElement:() => ({textContent:''})},
    Event: class { constructor(type) { this.type = type; } },
    window:{__TAURI__:{core:{invoke:async (command,args) => { calls.push({command,args}); return replies[command]?.(args); }}}}
  });
  return {element, calls};
}
const settle = async () => { for (let i=0; i<4; i++) await new Promise(setImmediate); };

test('initial inspection and cancelled preview never apply configuration or query providers', async () => {
  const h=harness(); await settle();
  assert.deepEqual(h.calls.map(c=>c.command), ['inspect_setup']);
  await h.element('claude-setup-enable').click();
  assert.equal(h.element('claude-setup-dialog').open,true);
  assert.equal(h.calls.some(c=>c.command==='apply_claude_setup'),false);
  await h.element('claude-setup-cancel').click(); await settle();
  assert.equal(h.calls.at(-1).command,'cancel_claude_setup');
  assert.equal(h.element('claude-setup-before').textContent,'');
  assert.equal(h.element('claude-setup-path').textContent,'');
});

test('explicit apply saves the resulting source and starts monitoring only after success', async () => {
  const h=harness(); let watch=0; h.element('claude-watch').addEventListener('click',()=>watch++);
  await settle(); await h.element('claude-setup-enable').click(); await h.element('claude-setup-apply').click();
  assert.equal(watch,1);
  assert.equal(h.element('claude-path').value,'C:/report.json');
  assert.deepEqual(h.calls.filter(c=>c.command==='save_claude_path').map(c=>c.args.path),['C:/report.json']);
  assert.equal(h.element('claude-setup-apply').disabled,true);
});

test('a configuration conflict does not save a source or start monitoring', async () => {
  const h=harness({apply_claude_setup:()=>{throw new Error('configuration changed');}});
  let watch=0; h.element('claude-watch').addEventListener('click',()=>watch++);
  await settle(); await h.element('claude-setup-enable').click(); await h.element('claude-setup-apply').click();
  assert.equal(watch,0);
  assert.equal(h.calls.some(c=>c.command==='save_claude_path'),false);
  assert.match(h.element('claude-setup-result').textContent,/configuration changed/);
});

test('scanning preserves an explicit ADB path', async () => {
  const h=harness(); h.element('adb-path').value='D:/custom/adb.exe'; await settle();
  assert.equal(h.element('adb-path').value,'D:/custom/adb.exe');
});
