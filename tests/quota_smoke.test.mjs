import test from 'node:test';
import assert from 'node:assert/strict';
import {codexWindows,copilotWindows,claudeWindows,antigravityWindows,confirmClaudeTrust} from '../scripts/quota-smoke.mjs';

test('Claude trust confirmation waits for the interactive prompt and rechecks it',() => {
  const writes=[]; let callback;
  let screen='> Yes, I trust this folder';
  confirmClaudeTrust(value=>writes.push(value),()=>screen,fn=>{callback=fn;});
  assert.deepEqual(writes,[]);
  callback(); assert.deepEqual(writes,['\r']);
  confirmClaudeTrust(value=>writes.push(value),()=>screen,fn=>{callback=fn;});
  screen='Main prompt'; callback();
  assert.deepEqual(writes,['\r']);
});
test('Claude fresh-directory default No is explicitly changed to Yes before Enter',() => {
  let screen='> No, exit\n  Yes, I trust this folder';
  const writes=[],callbacks=[];
  confirmClaudeTrust(value=>writes.push(value),()=>screen,fn=>callbacks.push(fn));
  callbacks.shift()();
  assert.deepEqual(writes,['\u001b[B']);
  screen='  No, exit\n> Yes, I trust this folder';
  callbacks.shift()();
  assert.deepEqual(writes,['\u001b[B','\r']);
});

test('Codex accepts numeric zero but not missing or string quota',() => {
  assert.equal(codexWindows({rateLimits:{primary:{usedPercent:0}}})[0].remaining_percent,100);
  assert.deepEqual(codexWindows({rateLimits:{primary:{usedPercent:null}}}),[]);
  assert.deepEqual(codexWindows({rateLimits:{primary:{usedPercent:'10'}}}),[]);
});
test('Copilot ignores zero-entitlement placeholders and never infers quota from tokens',() => {
  assert.deepEqual(copilotWindows({quotaSnapshots:{chat:{remainingPercentage:100,entitlementRequests:0}}}),[]);
  assert.deepEqual(copilotWindows({usage:{tokens:100}}),[]);
  assert.equal(copilotWindows({quotaSnapshots:{premium:{remainingPercentage:24.7,entitlementRequests:1500}}})[0].remaining_percent,24.7);
});
test('Claude requires both labelled percentages; missing reset is unknown, not missing quota',() => {
  const screen = 'Current session\n████ 6% used\nResets 3am (Etc/GMT-8)\nCurrent week (all models)\n████ 92% used\nResets Sep 10, 3pm (Etc/GMT-8)';
  assert.deepEqual(claudeWindows(screen).map(q => q.remaining_percent),[94,8]);
  assert.deepEqual(claudeWindows('Context window 6% used'),[]);
  assert.deepEqual(claudeWindows(screen.replace('92%','192%')),[]);
  assert.equal(claudeWindows(screen.replace('Resets Sep 10, 3pm (Etc/GMT-8)',''))[1].reset_display,null);
  const fresh='Current session\n0% used\nCurrent week (all models)\n██ 98% used\nResets Sep 10, 3pm';
  assert.deepEqual(claudeWindows(fresh).map(q=>q.remaining_percent),[100,2]);
});
test('Antigravity fails closed on changed/localized output and invalid values',() => {
  const row = 'Gemini Models\tWeekly Limit Remaining\t58%\t2026-09-11T02:36:20Z';
  assert.equal(antigravityWindows(row)[0].remaining_percent,58);
  for (const text of [row.replace('58%','158%'),row.replace('Remaining','Used'),row+'\nUnexpected',row.replace('2026-09-11T02:36:20Z','unknown')]) {
    assert.deepEqual(antigravityWindows(text),[]);
  }
});
