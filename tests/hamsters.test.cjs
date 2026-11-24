const {test} = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
function load(storage) {
  const context = vm.createContext({window:{localStorage:storage}});
  for (const name of ['pets.js','hamsters.js']) vm.runInContext(fs.readFileSync(path.join(__dirname,'../desktop-p0/ui',name),'utf8'),context);
  return context;
}
function memory(initial={}) {
  const map = new Map(Object.entries(initial));
  return {map,getItem:key=>map.has(key)?map.get(key):null,setItem:(key,value)=>map.set(key,value)};
}
const KEY='agentmeter.hamsters.v1', STEP=12000;

test('core providers start with different hamsters and unknown providers get a stable one', () => {
  const h = load(memory());
  const breeds = ['codex','claude','copilot','antigravity'].map(id=>h.hamsterBreedFor(id));
  assert.equal(new Set(breeds).size,4);
  for (const id of ['future-agent','constructor','__proto__']) assert.equal(vm.runInContext(`isHamsterBreed(hamsterBreedFor('${id}'))`,h),true);
});

test('a chosen hamster is saved per provider and seen by another window', () => {
  const storage = memory(), main = load(storage), hud = load(storage);
  assert.equal(hud.hamsterBreedFor('claude'),'pudding');
  assert.equal(main.chooseHamster('claude','panda'),true);
  assert.equal(main.hamsterBreedFor('claude'),'panda');
  assert.equal(hud.hamsterBreedFor('claude'),'panda');
  assert.equal(hud.hamsterBreedFor('codex'),'golden');
});

test('corrupt choices fall back to defaults and blocked storage still applies for this run', () => {
  const corrupt = load(memory({[KEY]:'{"claude":"dragon","codex":"snow","copilot":1}'}));
  assert.equal(corrupt.hamsterBreedFor('claude'),'pudding');
  assert.equal(corrupt.hamsterBreedFor('codex'),'snow');
  assert.equal(load(memory({[KEY]:'not json'})).hamsterBreedFor('codex'),'golden');
  const blocked = load({getItem(){throw new Error('blocked');},setItem(){throw new Error('blocked');}});
  assert.equal(blocked.chooseHamster('codex','robo'),false);
  assert.equal(blocked.hamsterBreedFor('codex'),'robo');
});

test('a hamster is in the wheel exactly when quota is being consumed', () => {
  const h = load(memory());
  for (let i=0;i<16;i++) {
    assert.equal(h.hamsterActivity('claude',{state:'running'},12,i*STEP).act,'run');
    assert.equal(h.hamsterActivity('claude',{state:'learning'},3,i*STEP).act,'run');
    assert.ok(['stroll','eat','rest','sleep'].includes(h.hamsterActivity('claude',{state:'idle'},0,i*STEP).act));
    assert.equal(h.hamsterActivity('claude',{state:'exhausted'},0,i*STEP).act,'tired');
  }
});

test('the idle routine strolls, eats, rests and sleeps, moving only while strolling and facing its path', () => {
  const h = load(memory()), seen = new Set();
  let facing = null;
  for (let i=1;i<=24;i++) {
    const before = h.hamsterActivity('codex',null,0,(i-1)*STEP), now = h.hamsterActivity('codex',null,0,i*STEP);
    seen.add(now.act);
    assert.ok(now.x>=0&&now.x<=1);
    if (now.x!==before.x) {
      assert.equal(now.act,'stroll');
      facing = now.x<before.x?'left':'right';
    }
    if (facing) assert.equal(now.facing,facing);
  }
  assert.deepEqual([...seen].sort(),['eat','rest','sleep','stroll']);
});

test('one exhausted window does not stop a hamster while another window is consumed', () => {
  const h = load(memory());
  assert.equal(h.hamsterActivity('codex',{state:'exhausted'},20,0).act,'run');
  assert.equal(h.petMood({state:'exhausted'},20),'run');
});
