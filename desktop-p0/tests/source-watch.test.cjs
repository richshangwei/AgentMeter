const {test} = require('node:test');
const assert = require('node:assert/strict');
const {readFileSync} = require('node:fs');
const vm = require('node:vm');
const context = vm.createContext({});
vm.runInContext(readFileSync(require('node:path').join(__dirname, '../ui/source-watch.js'), 'utf8'), context);
function harness(read) {
  const timers = new Map(), values = [], errors = [];
  let id = 0;
  const watch = context.createSourceWatch({read, success: v => values.push(v), failure: e => errors.push(e),
    schedule: (fn, ms) => {timers.set(++id, {fn, ms}); return id;}, cancel: key => timers.delete(key)});
  async function next() {const [key, timer] = timers.entries().next().value; timers.delete(key); await timer.fn();}
  return {watch, timers, values, errors, next};
}
test('repeats after completion and stops without another read', async () => {
  let reads = 0;
  const h = harness(async path => `${path}:${++reads}`);
  await h.watch.start('chosen');
  assert.equal(h.timers.values().next().value.ms, 5000);
  await h.next();
  assert.deepEqual(h.values, ['chosen:1', 'chosen:2']);
  h.watch.stop(); assert.equal(h.timers.size, 0);
});
test('failure retries; a one-shot import does not poll', async () => {
  let reads = 0;
  const h = harness(async () => {if (++reads === 1) throw Error('missing'); return 'recovered';});
  await h.watch.start('chosen'); await h.next();
  assert.equal(h.errors.length, 1); assert.deepEqual(h.values, ['recovered']);
  await h.watch.start('chosen', false); assert.equal(h.timers.size, 0);
});
test('switching source suppresses old results and never overlaps reads', async () => {
  let resolveOld, reads = [];
  const h = harness(path => {reads.push(path); return path === 'old' ? new Promise(resolve => {resolveOld = resolve;}) : Promise.resolve('new');});
  const pending = h.watch.start('old');
  await h.watch.start('new');
  assert.deepEqual(reads, ['old']);
  resolveOld('old'); await pending; assert.deepEqual(h.values, []);
  await h.next(); assert.deepEqual(reads, ['old', 'new']); assert.deepEqual(h.values, ['new']);
  h.watch.stop();
});
test('stopping an in-flight read suppresses its result and retry', async () => {
  let resolve;
  const h = harness(() => new Promise(r => {resolve = r;}));
  const pending = h.watch.start('chosen'); h.watch.stop(); resolve('ignored'); await pending;
  assert.deepEqual(h.values, []); assert.equal(h.timers.size, 0);
});
