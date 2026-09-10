const {test} = require('node:test');
const assert = require('node:assert/strict');
const vm = require('node:vm');
const context = vm.createContext({});
vm.runInContext(require('node:fs').readFileSync(require('node:path').join(__dirname,'../tablet-ui/recovery.js'),'utf8'),context);
test('recreated client restores exchange CSRF only and can forget it', () => {
  const entries = new Map();
  const storage = {getItem:k=>entries.get(k),setItem:(k,v)=>entries.set(k,v),removeItem:k=>entries.delete(k)};
  const first = context.createPairRecovery(storage);
  assert.equal(first.save('a'.repeat(32)),true);
  assert.equal(entries.size,1);
  const restarted = context.createPairRecovery(storage);
  assert.equal(restarted.read(),'a'.repeat(32));
  restarted.clear(); assert.equal(first.read(),'');
});
test('invalid material and denied storage do not grant recovery', () => {
  const denied = context.createPairRecovery({getItem(){throw Error();},setItem(){throw Error();},removeItem(){throw Error();}});
  assert.equal(denied.read(),''); assert.equal(denied.save('a'.repeat(32)),false); denied.clear();
  assert.equal(denied.save('not-a-csrf'),false);
  const corrupt = context.createPairRecovery({getItem:()=> 'bad'});
  assert.equal(corrupt.read(),'');
});
