// 倉鼠樂園 theme: original inline-SVG hamsters that run in a wheel while quota is consumed
// and live outside it otherwise. Shares petNode/petPath/petStroke/petEllipse with pets.js.
const hamsterKey = 'agentmeter.hamsters.v1';
const hamsterBreeds = {
  golden:   { name: '黃金鼠', coat: '#e9a552', shade: '#c97f2e', belly: '#fff6e6', ink: '#4a2c12' },
  pudding:  { name: '布丁鼠', coat: '#f9dc94', shade: '#e5bb60', belly: '#fffaf0', ink: '#5a3d1c' },
  stripe:   { name: '三線鼠', coat: '#b3a294', shade: '#8d7b6d', belly: '#fbf6f0', ink: '#3b2d24', stripe: '#5d4c40' },
  snow:     { name: '銀狐鼠', coat: '#fdfdfb', shade: '#d9d7d3', belly: '#ffffff', ink: '#3d3a3a', stripe: '#b9b5b2', outline: true },
  robo:     { name: '老公公鼠', coat: '#dcb27c', shade: '#bb8c52', belly: '#ffffff', ink: '#402a15', brow: true, size: .9 },
  panda:    { name: '熊貓鼠', coat: '#fffdf8', shade: '#dcd6cc', belly: '#ffffff', ink: '#2f2b2b', patch: '#4b4545', outline: true },
  sapphire: { name: '藍寶石鼠', coat: '#a9b4c6', shade: '#8390a6', belly: '#f7f9fc', ink: '#2c3442', stripe: '#6c7990' },
  milktea:  { name: '奶茶鼠', coat: '#d3ab8c', shade: '#b2886a', belly: '#fff3e6', ink: '#452c1e' },
};
const hamsterDefaults = { codex: 'golden', claude: 'pudding', copilot: 'snow', antigravity: 'stripe' };
const isHamsterBreed = value => typeof value === 'string' && Object.prototype.hasOwnProperty.call(hamsterBreeds, value);
function hamsterHash(text) {
  let hash = 0;
  for (const char of String(text)) hash = (hash * 31 + char.codePointAt(0)) >>> 0;
  return hash;
}
function hamsterDefault(provider) {
  if (Object.prototype.hasOwnProperty.call(hamsterDefaults, provider)) return hamsterDefaults[provider];
  const ids = Object.keys(hamsterBreeds);
  return ids[hamsterHash(provider) % ids.length];
}
function normalizeHamsterChoices(value) {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return {};
  return Object.fromEntries(Object.entries(value).filter(([, breed]) => isHamsterBreed(breed)));
}
let hamsterCache = { raw: null, value: {} };
function hamsterChoices() {
  try {
    const raw = window.localStorage?.getItem(hamsterKey) ?? null;
    if (raw !== hamsterCache.raw) hamsterCache = { raw, value: normalizeHamsterChoices(raw === null ? {} : JSON.parse(raw)) };
  } catch { /* storage unavailable or corrupt: keep the choices already in memory */ }
  return hamsterCache.value;
}
function hamsterBreedFor(provider) {
  const choices = hamsterChoices();
  return Object.prototype.hasOwnProperty.call(choices, provider) ? choices[provider] : hamsterDefault(provider);
}
// Returns false when the choice only applies to this run.
function chooseHamster(provider, breed) {
  const value = { ...hamsterChoices(), [provider]: breed }, raw = JSON.stringify(value);
  try { window.localStorage.setItem(hamsterKey, raw); hamsterCache = { raw, value }; return true; }
  catch { hamsterCache = { raw: hamsterCache.raw, value }; return false; }
}

// What a hamster does outside the wheel. x is the position in the yard: 0 beside the wheel, 1 at the house.
const hamsterStepMs = 12000;
const hamsterRoutine = [
  { act: 'stroll', x: .5 }, { act: 'eat', x: .5 }, { act: 'stroll', x: .12 }, { act: 'rest', x: .12 },
  { act: 'stroll', x: 1 }, { act: 'sleep', x: 1 }, { act: 'sleep', x: 1 }, { act: 'sleep', x: 1 },
];
const hamsterActText = { stroll: '散步中', eat: '吃點心中', rest: '休息中', sleep: '睡覺中' };
function hamsterActivity(provider, entry, rate, now) {
  // Any window still being consumed wins over another window that is already used up.
  if (rate > 0) return { act: 'run', x: 0, facing: 'right' };
  if (entry?.state === 'exhausted') return { act: 'tired', x: 0, facing: 'right' };
  // Offset per provider so the hamsters do not move in lockstep.
  const length = hamsterRoutine.length, step = (Math.floor(now / hamsterStepMs) + hamsterHash(provider)) % length;
  const current = hamsterRoutine[step];
  // Face along the last move, so a hamster that just walked left keeps facing left while it rests.
  let from = current;
  for (let back = 1; back < length && from.x === current.x; back++) from = hamsterRoutine[(step + length - back) % length];
  return { ...current, facing: current.x < from.x ? 'left' : 'right' };
}

function hamsterFoot(x, y, fill, name) { return petEllipse(x, y, 3.6, 2.4, fill, { class: 'ham-foot ' + name }); }
function createHamster(breed) {
  const c = hamsterBreeds[breed], line = c.outline ? { stroke: c.shade, 'stroke-width': .9 } : {}, scale = c.size || 1;
  const back = [];
  if (c.stripe) back.push(petStroke('M8 19 Q17 8.6 30 10', c.stripe, 2.6));
  if (c.patch) back.push(petPath('M13.6 12.6 Q20 9.4 26 11.6 Q27 22 22 36.6 Q14 36.6 11.6 33 Q15 22 13.6 12.6 Z', c.patch));
  const head = [
    petEllipse(30.4, 9, 3.8, 3.8, c.patch || c.shade), petEllipse(30.6, 9.3, 2, 2, '#f7a6ac'),
    petEllipse(39.4, 9.4, 3.4, 3.6, c.patch || c.shade), petEllipse(39.4, 9.7, 1.7, 1.9, '#f7a6ac'),
    petEllipse(35, 19.6, 11, 10.4, c.coat, line),
    petEllipse(39.4, 24, 6, 4.8, c.belly), petEllipse(32, 25, 5.2, 4.6, c.belly),
  ];
  if (c.stripe) head.push(petStroke('M33.6 9.8 L35 13.4', c.stripe, 2));
  if (c.patch) head.push(petEllipse(36.2, 17.4, 3.6, 4.2, c.patch, { opacity: .35 }));
  if (c.brow) head.push(petEllipse(36.4, 13.6, 2.6, 1.5, '#ffffff'), petEllipse(42, 13.8, 1.8, 1.3, '#ffffff'));
  head.push(
    petEllipse(32.4, 23.6, 2.5, 1.7, '#ff9aa8', { opacity: .8 }),
    petNode('g', { class: 'ham-eyes-open' }, [petEllipse(36.4, 18, 2, 2.5, c.ink), petEllipse(37.1, 17, .75, .85, '#ffffff'), petEllipse(42.6, 18, 1.5, 2.3, c.ink), petEllipse(43, 17.1, .55, .7, '#ffffff')]),
    petNode('g', { class: 'ham-eyes-closed' }, [petStroke('M34.4 18 Q36.4 20.2 38.4 18 M41.2 18 Q42.6 20 44 18', c.ink, 1.2)]),
    petEllipse(43.4, 21.6, 1.5, 1.1, '#f0808f'),
    petStroke('M43.2 22.8 Q42.4 24.8 40 24.2', c.ink, .9),
    petStroke('M37 23.4 L31.6 22.6 M37 25 L32.2 25.6', c.ink, .5, { opacity: .4 }),
  );
  const body = petNode('g', { class: 'ham-bob' }, [
    petEllipse(5.4, 30, 2.4, 1.9, '#f2a7a0'),
    hamsterFoot(16, 37, c.shade, 'ham-foot-b'), hamsterFoot(33, 37, c.shade, 'ham-foot-a'),
    petEllipse(21, 24.4, 16, 13.2, c.coat, line),
    ...back,
    petEllipse(23, 31.4, 11, 6, c.belly),
    petNode('g', { class: 'ham-head' }, head),
    hamsterFoot(12, 37.2, '#f5b5a8', 'ham-foot-a'), hamsterFoot(29.6, 37.2, '#f5b5a8', 'ham-foot-b'),
    petNode('g', { class: 'ham-seed' }, [petPath('M43.4 24.4 Q47.6 25.6 46.8 30 Q42.6 29.6 43.4 24.4 Z', '#5b4a3c'), petStroke('M44.4 25.6 Q45.6 27 46 29', '#f2e6d0', .8), petEllipse(42.6, 28.6, 2.2, 1.8, '#f5b5a8')]),
  ]);
  return petNode('svg', { viewBox: '0 -6 50 46', class: 'ham', 'data-breed': breed, 'aria-hidden': 'true', focusable: 'false' }, [
    petEllipse(25, 38.4, 17, 1.4, 'currentColor', { class: 'ham-shadow', opacity: .14 }),
    petNode('g', { class: 'ham-turn' }, [petNode('g', { transform: `translate(${25 - 25 * scale} ${38 - 38 * scale}) scale(${scale})` }, [body])]),
    petNode('g', { class: 'ham-sleep' }, [[40, 4, 4], [45, -3, 5]].map(([x, y, size], i) => petStroke(`M${x} ${y} h${size} l-${size} ${size} h${size}`, '#8d7bd0', 1.4, { class: 'ham-z ham-z' + i }))),
    petPath('M25 1 Q22 5.4 25 7 Q28 5.4 25 1 Z', '#5bb8f5', { class: 'ham-sweat' }),
  ]);
}
function wheelSvg(part) {
  const ring = (r, attributes) => petNode('circle', { cx: 50, cy: 50, r, fill: 'none', ...attributes });
  if (part === 'back') {
    const spokes = [0, 45, 90, 135].map(angle => petNode('line', { x1: 50, y1: 8, x2: 50, y2: 92, stroke: '#e2b57e', 'stroke-width': 2.4, transform: `rotate(${angle} 50 50)` }));
    return petNode('svg', { viewBox: '0 0 100 104', class: 'wheel wheel-back' }, [
      petPath('M50 50 L26 103 H34 L50 66 L66 103 H74 Z', '#c48d55'),
      ring(44, { fill: '#fff8ea' }),
      petNode('g', { class: 'wheel-spin' }, spokes),
      ring(4.5, { fill: '#b98450' }),
    ]);
  }
  // The rim doubles as the remaining-quota gauge; its rungs turn with the wheel.
  return petNode('svg', { viewBox: '0 0 100 104', class: 'wheel wheel-front' }, [
    ring(45, { stroke: '#ecd6b6', 'stroke-width': 8 }),
    ring(45, { class: 'wheel-gauge', 'stroke-width': 8, pathLength: 100, transform: 'rotate(-90 50 50)' }),
    ring(41.5, { class: 'wheel-spin', stroke: '#b98450', 'stroke-width': 3, 'stroke-dasharray': '2 11.04' }),
  ]);
}
function createHabitat(furnished) {
  const habitat = document.createElement('div');
  habitat.className = 'habitat';
  habitat.setAttribute('aria-hidden', 'true');
  const spot = document.createElement('div');
  spot.className = 'ham-spot';
  if (furnished) habitat.append(
    petNode('svg', { viewBox: '0 0 60 50', class: 'ham-house' }, [petPath('M8 24 H52 V50 H8 Z', '#e7b479'), petPath('M3 26 L30 4 L57 26 L52 29 L30 11 L8 29 Z', '#c4673a'), petPath('M19 50 V40 A11 11 0 0 1 41 40 V50 Z', '#7a4c2a'), petEllipse(30, 22, 3.4, 3.4, '#fff0d2')]),
    petNode('svg', { viewBox: '0 0 40 22', class: 'ham-bowl' }, [petEllipse(14, 8, 4, 3, '#5b4a3c'), petEllipse(21, 6.4, 4, 3, '#6c5745'), petEllipse(27, 8.4, 4, 3, '#5b4a3c'), petPath('M3 9 H37 L33 21 H7 Z', '#7fb7d9'), petPath('M3 9 H37 L36 12 H4 Z', '#a6d2ea')]),
  );
  habitat.append(wheelSvg('back'), spot, wheelSvg('front'));
  return habitat;
}
function updateHabitat(habitat, provider, activity, remaining, stride) {
  const spot = habitat.querySelector('.ham-spot'), breed = hamsterBreedFor(provider);
  if (spot.firstElementChild?.dataset.breed !== breed) spot.replaceChildren(createHamster(breed));
  for (const name of ['act', 'facing']) if (habitat.dataset[name] !== activity[name]) habitat.dataset[name] = activity[name];
  for (const [name, value] of [['--x', String(activity.x)], ['--stride', stride + 's'], ['--remaining', String(remaining ?? 0)]])
    if (habitat.style.getPropertyValue(name) !== value) habitat.style.setProperty(name, value);
}

function renderHamsterOptions() {
  const host = document.getElementById('hamster-options');
  if (!host) return;
  host.hidden = document.documentElement.dataset.theme !== 'hamster';
  if (host.hidden) return;
  host.replaceChildren(...desktopAvailable.map(provider => {
    const row = document.createElement('label');
    row.className = 'hamster-option';
    const preview = document.createElement('span');
    preview.className = 'hamster-preview';
    const name = document.createElement('span');
    name.textContent = desktopProviderName(provider);
    const select = document.createElement('select');
    select.setAttribute('aria-label', desktopProviderName(provider) + ' 的倉鼠品種');
    select.append(...Object.entries(hamsterBreeds).map(([id, breed]) => {
      const option = document.createElement('option');
      option.value = id;
      option.textContent = breed.name;
      return option;
    }));
    select.value = hamsterBreedFor(provider);
    preview.append(createHamster(select.value));
    select.addEventListener('change', () => {
      const saved = chooseHamster(provider, select.value);
      preview.replaceChildren(createHamster(select.value));
      document.getElementById('theme-status').textContent = saved ? '' : '無法儲存倉鼠選擇，本次仍會套用。';
      refreshLanes();
    });
    row.append(preview, name, select);
    return row;
  }));
}
