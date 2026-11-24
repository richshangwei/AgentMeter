// Per-provider lane: depletion estimate in every theme, plus a running pet in the 可愛動物 theme.
let lastLaneSnapshot = null;
function ensureLane(provider) {
  const card = document.getElementById(provider + '-card');
  if (!card) return null;
  let lane = card.querySelector('.run-lane');
  if (lane) return lane;
  lane = document.createElement('div');
  lane.className = 'run-lane';
  lane.hidden = true;
  const track = document.createElement('div');
  track.className = 'lane-track';
  track.setAttribute('aria-hidden', 'true');
  const runner = document.createElement('div');
  runner.className = 'lane-runner';
  runner.append(createPet(provider));
  track.append(runner);
  const text = document.createElement('p');
  text.className = 'lane-text';
  lane.append(track, text);
  card.insertBefore(lane, card.querySelector('.card-foot'));
  return lane;
}
// Providers may report several windows under one label; the remaining percent tells them apart.
function laneWindowLabel(entry, windows) {
  const label = quotaLabel(entry.item);
  return windows.filter(other => quotaLabel(other.item) === label).length > 1 ? `${label}（剩 ${displayPercent(forecastRemaining(entry.item))}%）` : label;
}
// 倉鼠樂園: every quota gauge is a wheel with its own hamster, driven by that quota alone.
function renderPlaygrounds(state, windows, now) {
  const tiles = document.querySelectorAll('#' + CSS.escape(state.provider) + '-quota > .window:not(.usage-row)');
  (state.quota_windows || []).forEach((item, index) => {
    const tile = tiles[index];
    if (!tile) return;
    let habitat = tile.querySelector('.habitat');
    if (!habitat) tile.append(habitat = createHabitat(true));
    const entry = windows.find(candidate => candidate.item === item) || null, rate = entry?.rate || 0;
    const activity = hamsterActivity(state.provider + '/' + index, entry, rate, now);
    updateHabitat(habitat, state.provider, activity, forecastRemaining(item), petStrideSeconds(rate));
  });
}
function renderLanes(snapshot) {
  lastLaneSnapshot = snapshot;
  const now = Date.now(), theme = document.documentElement.dataset.theme, pets = theme === 'pets', hamster = theme === 'hamster';
  const samples = storeQuotaSamples(snapshot);
  for (const state of snapshot.provider_states || []) {
    if (!validDesktopProviderId(state?.provider)) continue;
    const lane = ensureLane(state.provider);
    if (!lane) continue;
    const { primary, windows, rate } = providerForecast(samples, state, now);
    if (hamster) renderPlaygrounds(state, windows, now);
    const message = forecastText(primary, now), hidden = !primary || (!pets && !message);
    lane.hidden = hidden;
    lane.parentElement.dataset.lane = String(!hidden);
    if (hidden) continue;
    const mood = petMood(primary, rate);
    const label = windows.length > 1 && message && primary.state !== 'learning' ? ' · ' + laneWindowLabel(primary, windows) : '';
    // One reading cannot show whether quota is being consumed.
    const observed = windows.some(entry => entry.observed > 1);
    const text = message ? message + label : petKinds[petKindFor(state.provider)].name + (observed ? '休息中，未偵測到消耗' : '正在觀察用量…');
    lane.dataset.level = forecastLevel(primary);
    const node = lane.querySelector('.lane-text');
    if (node.textContent !== text) node.textContent = text;
    const title = windows.filter(entry => forecastText(entry, now)).map(entry => [laneWindowLabel(entry, windows) + '：' + forecastText(entry, now), forecastBasis(entry)].filter(Boolean).join('\n　')).join('\n')
      || '連續觀測到額度下降後，才會估算多久用完。';
    if (lane.title !== title) lane.title = title;
    if (pets) {
      // The pet runs towards the finish line as the quota is used up.
      lane.style.setProperty('--used', String(100 - forecastRemaining(primary.item)));
      lane.style.setProperty('--ground', petStrideSeconds(rate) * 2 + 's');
      lane.dataset.mood = mood;
      updatePet(lane.querySelector('.pet'), mood, rate);
    }
  }
}
// Vertical room a lane takes from the quota region; keeps pagination decisions honest.
function desktopLaneHeight() {
  if (![...document.querySelectorAll('.monitor-card[data-lane="true"]')].length) return 0;
  return document.documentElement.dataset.theme === 'pets' ? 58 : 24;
}
// Animations stop while the window is hidden in the tray or minimised.
async function pollWindowVisibility() {
  try { document.documentElement.dataset.idle = String(await tabletCommand('window_visible') === false); }
  catch { document.documentElement.dataset.idle = 'false'; }
}
setInterval(pollWindowVisibility, 2000);
function refreshLanes() {
  if (!lastLaneSnapshot) return;
  renderLanes(lastLaneSnapshot);
  scheduleDesktopLayout();
}
document.addEventListener('agentmeter-theme', () => {
  renderHamsterOptions();
  refreshLanes();
});
