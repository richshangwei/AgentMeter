const {test} = require('node:test');
const assert = require('node:assert/strict');
const vm = require('node:vm');
const fs = require('node:fs');
const context = vm.createContext({});
vm.runInContext(fs.readFileSync(require('node:path').join(__dirname,'../tablet-ui/protocol.js'),'utf8'), context);
const snapshot = (stream_id, revision) => ({stream_id,revision,providers:['claude','codex','copilot','antigravity'].map(provider => ({provider}))});
test('only full fetch may switch streams; duplicates and late events are rejected', () => {
  const accept = context.createSnapshotGate();
  assert.equal(accept(snapshot('a',4)), false);
  assert.equal(accept(snapshot('a',4),true), true);
  assert.equal(accept(snapshot('a',4)), false);
  assert.equal(accept(snapshot('a',3)), false);
  assert.equal(accept(snapshot('a',5)), true);
  assert.equal(accept(snapshot('b',0),true), true);
  assert.equal(accept(snapshot('a',6)), false);
  assert.equal(accept(snapshot('b',1)), true);
});
test('partial snapshots do not advance the accepted revision', () => {
  const accept = context.createSnapshotGate();
  const broken = snapshot('a',2); broken.providers.pop();
  assert.throws(() => accept(broken,true));
  assert.equal(accept(snapshot('a',1),true), true);
});
test('fragmented CRLF, multiline data, and heartbeat do not invent revisions', () => {
  const values = []; let events = 0;
  const parse = context.createEventParser(value => values.push(value), () => events++);
  parse(': heartbeat\r\n\r'); parse('\nevent: dashboard\r\ndata: {"x":\r\ndata: 1}\r\n\r\n');
  assert.equal(events,2); assert.equal(values.length,1); assert.equal(values[0].x,1);
});
test('invalid JSON and oversized unfinished events fail closed', () => {
  const parse = context.createEventParser(() => {}, () => {});
  assert.throws(() => parse('event: dashboard\ndata: bad\n\n'));
  assert.throws(() => parse('x'.repeat(262145)));
});
test('watchdog resets on complete events and stop cancels pending expiry', () => {
  let next=0, expired=0; const timers=new Map();
  const watch=context.createWatchdog(() => expired++, (fn,ms) => {assert.equal(ms,45000); timers.set(++next,fn);return next;}, id => timers.delete(id));
  watch.touch(); watch.touch(); assert.equal(timers.size,1);
  timers.values().next().value(); assert.equal(expired,1);
  watch.stop(); assert.equal(timers.size,0);
});
