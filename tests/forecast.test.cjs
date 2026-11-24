const {test} = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');

function load(storage) {
  const context = vm.createContext({window:{localStorage:storage}});
  vm.runInContext(fs.readFileSync(path.join(__dirname,'../desktop-p0/ui/forecast.js'),'utf8'),context);
  return context;
}
const f = load();
const MIN = 60000, HOUR = 60 * MIN;
const now = new Date(2026,10,24,14,0,0).getTime();
// values sampled every `every` minutes, the last one exactly at `now`
const series = (values, every = 2) => values.map((value,i)=>[now-(values.length-1-i)*every*MIN,value]);
const five = (remaining, resetInMs) => ({window:'five_hour',remaining_percent:remaining,resets_at:resetInMs==null?null:(now+resetInMs)/1000});

test('continuous consumption projects the exhaustion time from the observed pace', () => {
  const list = series([60,58,56,54,52,50],10); // 12% per hour
  const result = f.forecastQuota(list,five(50,4.9*HOUR),now);
  assert.equal(result.state,'running');
  assert.ok(Math.abs(result.rate-12)<1e-9);
  assert.ok(Math.abs(result.etaMs-50/12*HOUR)<1000);
  assert.equal(f.forecastText(result,now),'預估 4 小時 10 分後用完（18:10）');
  assert.equal(f.forecastShortText(result),'預估 4時10分後用完');
});

test('a reset that comes before exhaustion is reported as safe, not as running out', () => {
  const result = f.forecastQuota(series([60,58,56,54,52,50],10),five(50,HOUR),now);
  assert.equal(result.state,'safe');
  assert.equal(f.forecastText(result,now),'預估重設前不會用完');
});

test('the window length bounds the estimate when the reset time is unknown', () => {
  const slow = series([100,99,98],15); // 4%/h → 24.5 h left, far beyond a 5 hour window
  assert.equal(f.forecastQuota(slow,five(98,null),now).state,'safe');
  const monthly = {bucket_key:'premium_interactions:',remaining_percent:98,resets_at:null};
  const result = f.forecastQuota(slow,monthly,now);
  assert.equal(result.state,'running');
  assert.match(f.forecastBasis(result),/重設時間未知/);
});

test('no consumption, a single step, or stopped consumption never yields an estimate', () => {
  assert.equal(f.forecastQuota(series([50,50,50,50],10),five(50,HOUR),now).state,'idle');
  assert.equal(f.forecastQuota(series([51,51,50],10),five(50,HOUR),now).state,'learning');
  const stopped = series([60,58,56,...Array(20).fill(54)],4);
  assert.equal(f.forecastQuota(stopped,five(54,4*HOUR),now).state,'idle');
  assert.equal(f.forecastQuota([],five(54,HOUR),now).state,'idle');
  assert.equal(f.forecastQuota([],five(54,HOUR),now).observed,0);
  assert.equal(f.forecastQuota(series([54]),five(54,HOUR),now).observed,1);
  assert.equal(f.forecastQuota([[now-3*HOUR,54],[now,54]],five(54,HOUR),now).observed,1);
  assert.equal(f.forecastQuota(undefined,five(null,HOUR),now).state,'idle');
});

test('a quota reset or an observation gap discards the earlier pace', () => {
  assert.equal(f.forecastQuota(series([20,10,2,100,100,100],5),five(100,4*HOUR),now).state,'idle');
  const gap = [[now-5*HOUR,90],[now-4.9*HOUR,80],[now-2*HOUR,50],[now-1.9*HOUR,50]];
  assert.equal(f.forecastQuota(gap,five(50,HOUR),now).state,'idle');
});

test('idle time before a burst does not dilute the current pace', () => {
  const result = f.forecastQuota(series([80,80,80,80,80,75,70,65],5),five(65,4.9*HOUR),now);
  assert.ok(Math.abs(result.rate-60)<1e-9);
});

test('hourly polling and slow whole-percent weekly steps still produce an estimate', () => {
  const hourly = f.forecastQuota(series([80,70,60,50],60),five(50,4.95*HOUR),now);
  assert.equal(hourly.state,'safe'); // 5 h at 10%/h, reset just earlier
  const weekly = f.forecastQuota(series([40,40,39,39,39,38,38],15),{window:'seven_day',remaining_percent:38,resets_at:(now+6*24*HOUR)/1000},now);
  assert.equal(weekly.state,'running');
  assert.equal(f.forecastText(weekly,now),'預估 24 小時後用完（11/25 14:00）');
});

test('exhausted windows say so and include the reset clock', () => {
  const result = f.forecastQuota(series([2,1,0],5),five(0,2*HOUR),now);
  assert.equal(f.forecastText(result,now),'額度已用完，16:00 重設');
  assert.equal(f.forecastLevel(result),'danger');
});

test('reset times accept epoch seconds, ISO text and Claude wall-clock text', () => {
  const at = (month,day,hour,minute=0) => new Date(2026,month,day,hour,minute).getTime();
  assert.equal(f.forecastResetAt({resets_at:(now+HOUR)/1000},now),now+HOUR);
  assert.equal(f.forecastResetAt({resets_at:new Date(now+HOUR).toISOString()},now),now+HOUR);
  assert.equal(f.forecastResetAt({resets_at:(now-HOUR)/1000},now),null);
  assert.equal(f.forecastResetAt({window:'five_hour',reset_display:'4:30pm (Etc/GMT-8)'},now),at(10,24,16,30));
  assert.equal(f.parseResetDisplay('3am (Etc/GMT-8)',now),at(10,25,3));
  assert.equal(f.parseResetDisplay('Nov 27, 3pm (Etc/GMT-8)',now),at(10,27,15));
  assert.equal(f.parseResetDisplay('Nov 27 at 12am',now),at(10,27,0));
  assert.equal(f.parseResetDisplay('Jan 2, 3pm',now),new Date(2027,0,2,15).getTime());
  assert.equal(f.parseResetDisplay('2026/9/18 上午7:56:04 (Etc/GMT-8)',now),null);
  // A five-hour window cannot reset 13 hours from now: that wall clock is stale.
  assert.equal(f.forecastResetAt({window:'five_hour',reset_display:'3am'},now),null);
});

test('samples are recorded once per collection, skip failures and are bounded by age', () => {
  const snapshot = (at,remaining,extra={}) => ({provider_states:[{provider:'claude',collection_state:'ready',freshness:'fresh',collected_at:at,
    quota_windows:[{bucket_key:':five_hour',remaining_percent:remaining}],...extra}]});
  let samples = f.recordQuotaSamples({},snapshot(now,50));
  assert.equal(JSON.stringify(samples),JSON.stringify({'claude|:five_hour':[[now,50]]}));
  assert.equal(f.recordQuotaSamples(samples,snapshot(now,50)),samples);
  assert.equal(f.recordQuotaSamples(samples,snapshot(now+MIN,40,{collection_state:'error'})),samples);
  assert.equal(f.recordQuotaSamples(samples,snapshot(now+MIN,40,{freshness:'stale'})),samples);
  assert.equal(f.recordQuotaSamples(samples,snapshot(now+MIN,null)),samples);
  samples = f.recordQuotaSamples(samples,snapshot(now+7*HOUR,30));
  assert.equal(JSON.stringify(samples),JSON.stringify({'claude|:five_hour':[[now+7*HOUR,30]]}));
});

test('provider forecast surfaces the most urgent running window and ignores stale providers', () => {
  const provider = {provider:'codex',collection_state:'ready',freshness:'fresh',quota_windows:[
    {bucket_key:'codex:secondary',remaining_percent:80,resets_at:(now+5*24*HOUR)/1000},
    {bucket_key:'codex:primary',remaining_percent:20,resets_at:(now+4*HOUR)/1000},
    {bucket_key:'spark:primary',remaining_percent:0}]};
  const samples = {'codex|codex:secondary':series([82,81,80],10),'codex|codex:primary':series([30,25,20],10),'codex|spark:primary':series([0,0,0],10)};
  const result = f.providerForecast(samples,provider,now);
  assert.equal(result.primary.item.bucket_key,'codex:primary');
  assert.equal(result.windows.length,3);
  assert.ok(result.rate>20);
  assert.equal(f.providerForecast(samples,{...provider,freshness:'stale'},now).primary,null);
  assert.equal(f.providerForecast(samples,undefined,now).primary,null);
});

test('storage failures and corrupt saved samples fall back safely', () => {
  const snapshot = {provider_states:[{provider:'codex',collection_state:'ready',collected_at:now,quota_windows:[{bucket_key:'a',remaining_percent:9}]}]};
  const broken = load({getItem(){throw new Error('blocked');},setItem(){throw new Error('blocked');}});
  assert.equal(JSON.stringify(broken.storeQuotaSamples(snapshot)),JSON.stringify({'codex|a':[[now,9]]}));
  assert.equal(JSON.stringify(broken.storeQuotaSamples({provider_states:[{...snapshot.provider_states[0],collected_at:now+MIN}]})['codex|a'].length),'2');
  const corrupt = load({getItem:()=>'{"codex|a":"x","codex|b":[[1,2]]}',setItem(){}});
  assert.equal(JSON.stringify(corrupt.loadQuotaSamples()),JSON.stringify({'codex|b':[[1,2]]}));
});

test('durations round to honest precision', () => {
  assert.equal(f.forecastDuration(20000),'不到 1 分鐘');
  assert.equal(f.forecastDuration(7*MIN),'7 分鐘');
  assert.equal(f.forecastDuration(62*MIN),'1 小時');
  assert.equal(f.forecastDuration(50*HOUR),'2 天 2 小時');
  assert.equal(f.forecastDuration(50*HOUR,true),'2天2時');
});
