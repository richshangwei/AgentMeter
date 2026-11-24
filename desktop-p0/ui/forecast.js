// Estimates when a quota window runs out, from observed remaining-percent samples only.
// Never converts percentages into Token counts.
const quotaSamplesKey = 'agentmeter.quota-samples.v1';
const forecastMinute = 60000, forecastHour = 3600000, forecastDay = 86400000;
const forecastRetention = 6 * forecastHour; // samples kept per window
const forecastMaxGap = 90 * forecastMinute; // longer gaps mean the app was not observing
const forecastMinimumSpan = 5 * forecastMinute;
const forecastDropNoise = .05; // percentage points

function forecastWindowId(provider, item, index) {
  const name = item?.bucket_key || [item?.limit_id, item?.window, item?.label].filter(Boolean).join('/') || 'window-' + index;
  return provider + '|' + name;
}
function forecastRemaining(item) {
  const value = item?.remaining_percent;
  return typeof value === 'number' && Number.isFinite(value) ? Math.max(0, Math.min(100, value)) : null;
}
function forecastWindowMinutes(item) {
  const minutes = Number(item?.window_duration_mins);
  if (Number.isFinite(minutes) && minutes > 0) return minutes;
  const text = [item?.window, item?.bucket_key, item?.label].filter(Boolean).join(' ');
  if (/five[_ ]hour/i.test(text)) return 300;
  if (/seven[_ ]day|weekly/i.test(text)) return 10080;
  return null;
}
// Claude reports local wall-clock text such as "3am (Etc/GMT-8)" or "Sep 10, 3pm".
function parseResetDisplay(text, now) {
  const months = ['jan','feb','mar','apr','may','jun','jul','aug','sep','oct','nov','dec'];
  const match = /^(?:([a-z]{3})[a-z]* (\d{1,2})(?:,| at)? )?(\d{1,2})(?::(\d{2}))?\s*([ap])m\b/i.exec(String(text || '').trim());
  if (!match) return null;
  const hour = Number(match[3]) % 12 + (match[5].toLowerCase() === 'p' ? 12 : 0), minute = Number(match[4] || 0);
  if (Number(match[3]) < 1 || Number(match[3]) > 12 || minute > 59) return null;
  const date = new Date(now);
  date.setHours(hour, minute, 0, 0);
  if (!match[1]) {
    if (date.getTime() <= now) date.setDate(date.getDate() + 1);
    return date.getTime();
  }
  const month = months.indexOf(match[1].toLowerCase());
  if (month < 0) return null;
  date.setMonth(month, Number(match[2]));
  if (date.getTime() < now - 180 * forecastDay) date.setFullYear(date.getFullYear() + 1); // December → January
  return date.getTime();
}
function forecastResetAt(item, now) {
  const raw = item?.resets_at;
  let value = null;
  if (typeof raw === 'number') value = raw > 1e11 ? raw : raw * 1000;
  else if (typeof raw === 'string' && raw) value = Date.parse(raw);
  else if (raw == null) value = parseResetDisplay(item?.reset_display, now);
  if (!Number.isFinite(value) || value <= now) return null;
  const minutes = forecastWindowMinutes(item);
  return minutes && value - now > (minutes + 5) * forecastMinute ? null : value;
}

// Returns a new sample map, or the same object when nothing changed.
function recordQuotaSamples(samples, snapshot) {
  const base = samples && typeof samples === 'object' && !Array.isArray(samples) ? samples : {};
  let next = null;
  for (const state of snapshot?.provider_states || []) {
    const at = Number(state?.collected_at);
    if (!state || state.collection_state === 'error' || state.freshness === 'stale' || !(at > 0)) continue;
    (state.quota_windows || []).forEach((item, index) => {
      const remaining = forecastRemaining(item);
      if (remaining === null) return;
      const id = forecastWindowId(state.provider, item, index);
      const list = Array.isArray((next || base)[id]) ? (next || base)[id] : [];
      if (list.length && at <= list[list.length - 1][0]) return;
      if (!next) next = { ...base };
      next[id] = [...list.filter(sample => at - sample[0] <= forecastRetention), [at, remaining]];
    });
  }
  if (!next) return base;
  const newest = Math.max(...Object.values(next).map(list => list.length ? list[list.length - 1][0] : 0));
  for (const id of Object.keys(next)) {
    const list = next[id];
    if (!list.length || newest - list[list.length - 1][0] > forecastRetention) delete next[id];
  }
  return next;
}
function validQuotaSamples(value) {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return {};
  return Object.fromEntries(Object.entries(value).filter(([, list]) => Array.isArray(list)
    && list.every(sample => Array.isArray(sample) && Number.isFinite(sample[0]) && Number.isFinite(sample[1]))));
}

// Only the current, uninterrupted cycle counts: cut at resets and observation gaps.
function currentCycle(list) {
  let start = 0;
  for (let i = 1; i < list.length; i++) {
    if (list[i][1] - list[i - 1][1] > 1 || list[i][0] - list[i - 1][0] > forecastMaxGap) start = i;
  }
  return list.slice(start);
}
function forecastQuota(list, item, now) {
  const remaining = forecastRemaining(item);
  const resetAt = forecastResetAt(item, now), minutes = forecastWindowMinutes(item);
  const result = { state: 'idle', rate: 0, etaMs: null, exhaustAt: null, resetAt, basisMs: 0, observed: 0 };
  if (remaining === null) return result;
  if (remaining === 0) return { ...result, state: 'exhausted' };
  const cycle = currentCycle(Array.isArray(list) ? list : []);
  const last = cycle[cycle.length - 1];
  if (!last || now - last[0] > forecastMaxGap) return result;
  result.observed = cycle.length;
  const longWindow = minutes === null ? resetAt === null || resetAt - now > forecastDay : minutes >= 1440;
  const lookback = longWindow ? forecastRetention : forecastHour;
  // Slow polling may leave too few points inside the lookback; keep at least four.
  let recent = cycle.filter(sample => last[0] - sample[0] <= lookback);
  if (recent.length < 4) recent = cycle.slice(-4);
  const drops = [];
  for (let i = 1; i < recent.length; i++) if (recent[i - 1][1] - recent[i][1] >= forecastDropNoise) drops.push(i);
  if (!drops.length) return result;
  const first = recent[drops[0] - 1], lastDrop = recent[drops[drops.length - 1]];
  const span = last[0] - first[0], used = first[1] - last[1];
  if (used <= 0) return result;
  // Still consuming? Whole-percent sources step slowly, so scale with the observed step interval.
  const gap = span / Math.max(1, recent.length - drops[0]);
  const step = (lastDrop[0] - first[0]) / drops.length;
  const active = Math.min(3 * forecastHour, Math.max(20 * forecastMinute, 3 * gap, 1.5 * step));
  if (now - lastDrop[0] > active) return result;
  result.rate = used / Math.max(span, forecastMinimumSpan) * forecastHour;
  result.basisMs = span;
  if (drops.length < 2 || span < forecastMinimumSpan) return { ...result, state: 'learning' };
  result.exhaustAt = last[0] + last[1] / result.rate * forecastHour;
  result.etaMs = Math.max(0, result.exhaustAt - now);
  const resetsFirst = resetAt !== null ? resetAt <= result.exhaustAt : minutes !== null && result.etaMs > minutes * forecastMinute;
  return { ...result, state: resetsFirst ? 'safe' : 'running' };
}

const forecastOrder = ['running', 'exhausted', 'safe', 'learning', 'idle'];
function providerForecast(samples, state, now) {
  const failed = !state || state.collection_state === 'error' || state.freshness === 'stale';
  const windows = failed ? [] : (state.quota_windows || []).map((item, index) => ({
    item, ...forecastQuota(samples?.[forecastWindowId(state.provider, item, index)], item, now),
  })).filter(entry => forecastRemaining(entry.item) !== null);
  const sorted = [...windows].sort((a, b) => forecastOrder.indexOf(a.state) - forecastOrder.indexOf(b.state)
    || (a.etaMs ?? Infinity) - (b.etaMs ?? Infinity));
  return { primary: sorted[0] || null, windows, rate: Math.max(0, ...windows.map(entry => entry.rate)) };
}

function forecastRounded(ms) {
  const step = ms >= 10 * forecastHour ? 30 : ms >= forecastHour ? 5 : 1;
  return Math.round(ms / forecastMinute / step) * step; // minutes
}
function forecastDuration(ms, compact = false) {
  const total = forecastRounded(ms);
  if (total < 1) return compact ? '<1分' : '不到 1 分鐘';
  const days = Math.floor(total / 1440), hours = Math.floor(total % 1440 / 60), minutes = total % 60;
  const parts = total >= 2880 ? [[days, '天', '天'], [hours, '小時', '時']]
    : total >= 60 ? [[days * 24 + hours, '小時', '時'], [minutes, '分', '分']] : [[minutes, '分鐘', '分']];
  return parts.filter(([value]) => value > 0).map(([value, full, short]) => compact ? value + short : `${value} ${full}`).join(compact ? '' : ' ');
}
function forecastClock(at, now) {
  const date = new Date(at), sameDay = date.toDateString() === new Date(now).toDateString();
  const clock = String(date.getHours()).padStart(2, '0') + ':' + String(date.getMinutes()).padStart(2, '0');
  return sameDay ? clock : `${date.getMonth() + 1}/${date.getDate()} ${clock}`;
}
function forecastLevel(entry) {
  if (!entry) return 'none';
  if (entry.state === 'exhausted') return 'danger';
  if (entry.state === 'running') return entry.etaMs < 30 * forecastMinute ? 'danger' : 'warning';
  return entry.state === 'safe' ? 'good' : 'none';
}
function forecastText(entry, now) {
  if (entry?.state === 'exhausted') return entry.resetAt ? `額度已用完，${forecastClock(entry.resetAt, now)} 重設` : '額度已用完';
  if (entry?.state === 'running') {
    const at = now + forecastRounded(entry.etaMs) * forecastMinute;
    return entry.etaMs < forecastMinute ? '預估即將用完' : `預估 ${forecastDuration(entry.etaMs)}後用完（${forecastClock(at, now)}）`;
  }
  if (entry?.state === 'safe') return '預估重設前不會用完';
  if (entry?.state === 'learning') return '偵測到消耗，正在估算可用時間…';
  return '';
}
function forecastShortText(entry) {
  if (entry?.state === 'exhausted') return '已用完';
  if (entry?.state === 'running') return entry.etaMs < forecastMinute ? '即將用完' : '預估 ' + forecastDuration(entry.etaMs, true) + '後用完';
  if (entry?.state === 'safe') return '重設前夠用';
  return '';
}
function forecastBasis(entry) {
  if (entry?.state !== 'running' && entry?.state !== 'safe') return '';
  const rate = entry.rate >= 10 ? Math.round(entry.rate) : Math.round(entry.rate * 10) / 10 || '<0.1';
  return `依最近 ${forecastDuration(entry.basisMs)}的消耗估計，每小時約 ${rate}%${entry.resetAt === null && entry.state === 'running' ? '；重設時間未知' : ''}`;
}

// Storage: the main window records, the HUD only reads.
let quotaSampleCache = { raw: null, value: {} };
function loadQuotaSamples() {
  try {
    const raw = window.localStorage?.getItem(quotaSamplesKey) ?? null;
    if (raw !== quotaSampleCache.raw) quotaSampleCache = { raw, value: validQuotaSamples(raw === null ? {} : JSON.parse(raw)) };
  } catch { /* storage unavailable or corrupt: keep the in-memory samples */ }
  return quotaSampleCache.value;
}
function storeQuotaSamples(snapshot) {
  const before = loadQuotaSamples(), after = recordQuotaSamples(before, snapshot);
  if (after === before) return after;
  const raw = JSON.stringify(after);
  quotaSampleCache = { raw, value: after };
  try { window.localStorage?.setItem(quotaSamplesKey, raw); } catch { quotaSampleCache.raw = null; }
  return after;
}
