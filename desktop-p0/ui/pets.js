// Original side-view runner companions for the 可愛動物 theme. Pure inline SVG: no external assets.
const petKinds = {
  cat:     { name: '貓咪', coat: '#f6a94f', shade: '#e08a30', belly: '#fff1dc', ink: '#5a3a1e' },
  dog:     { name: '狗狗', coat: '#d9a066', shade: '#b97d44', belly: '#fff4e4', ink: '#4c321f' },
  rabbit:  { name: '兔兔', coat: '#fdf3f5', shade: '#eccfd8', belly: '#ffffff', ink: '#6b4a57' },
  hamster: { name: '倉鼠', coat: '#f3cf9a', shade: '#dba969', belly: '#fffaf0', ink: '#5c4327' },
  fox:     { name: '狐狸', coat: '#f0793f', shade: '#cf5a25', belly: '#fff3e8', ink: '#4a2717' },
};
const petByProvider = { codex: 'cat', claude: 'dog', copilot: 'rabbit', antigravity: 'hamster' };
function petKindFor(provider) {
  if (Object.prototype.hasOwnProperty.call(petByProvider, provider)) return petByProvider[provider];
  const kinds = Object.keys(petKinds);
  let hash = 0;
  for (const char of String(provider)) hash = (hash * 31 + char.codePointAt(0)) >>> 0;
  return kinds[hash % kinds.length];
}
// Faster consumption → shorter stride. Rate is quota percent per hour.
function petStrideSeconds(rate) {
  if (!(rate > 0)) return 0;
  return Math.round(Math.max(.16, .62 - .085 * Math.log2(1 + rate)) * 100) / 100;
}
function petMood(entry, rate) {
  if (rate > 0) return 'run';
  return entry?.state === 'exhausted' ? 'tired' : 'sleep';
}
function petNode(tag, attributes = {}, children = []) {
  const node = document.createElementNS('http://www.w3.org/2000/svg', tag);
  for (const [name, value] of Object.entries(attributes)) node.setAttribute(name, String(value));
  node.append(...children);
  return node;
}
const petPath = (d, fill, extra = {}) => petNode('path', { d, fill, ...extra });
const petStroke = (d, stroke, width, extra = {}) => petPath(d, 'none', { stroke, 'stroke-width': width, 'stroke-linecap': 'round', 'stroke-linejoin': 'round', ...extra });
const petEllipse = (cx, cy, rx, ry, fill, extra = {}) => petNode('ellipse', { cx, cy, rx, ry, fill, ...extra });
function petLeg(x, fill, name) {
  return petNode('g', { class: 'pet-leg ' + name }, [
    petNode('rect', { x: x - 2.6, y: 31, width: 5.2, height: 12, rx: 2.6, fill }),
    petEllipse(x + .8, 42, 3.4, 2, fill),
  ]);
}
function petTail(kind, c) {
  const tails = {
    cat: [petStroke('M13 27 C3 26 2 16 7 11', c.coat, 4.6), petStroke('M4.6 16 C4.8 14 5.6 12.4 7 11', c.shade, 4.6)],
    dog: [petStroke('M14 25 C5 23 6 12 13 14 C16 15.4 14 18.6 11.6 17.6', c.coat, 5), petStroke('M9.2 15.4 C10.4 13.8 12 13.6 13 14', c.belly, 4)],
    rabbit: [petEllipse(11, 27, 5, 4.8, '#ffffff', { stroke: c.shade, 'stroke-width': 1 })],
    hamster: [petEllipse(11, 31, 3, 2.2, '#f0a9a0')],
    fox: [petPath('M15 30 C4 32 -1 22 3 13 C7 16 14 19 17 25 Z', c.coat), petPath('M3 13 C1.4 17 1.4 20.6 2.6 23.6 C4.8 21.4 6.6 20 8 17.4 C6 16 4.2 14.6 3 13 Z', c.belly)],
  };
  return petNode('g', { class: 'pet-tail' }, tails[kind]);
}
function petEars(kind, c) {
  const pink = '#f9a8b4';
  return {
    cat: [petPath('M37 13 L38 2.5 L46 8 Z', c.coat), petPath('M39 10.5 L39.6 5.6 L43.6 8.4 Z', pink), petPath('M50 8 L57 2.5 L58.4 13 Z', c.coat), petPath('M52.4 8.6 L55.8 5.8 L56.6 11 Z', pink)],
    dog: [petPath('M38 12 C36.4 5 39 2.6 41 3 C44 4 46 7 46.4 9 Z', c.shade), petPath('M51 8 C53 4 56 2.4 57.6 3.4 C59.2 5 59 9 58 12.4 Z', c.shade)],
    rabbit: [petEllipse(43, 4, 3.4, 9, c.coat, { transform: 'rotate(-16 43 10)', stroke: c.shade, 'stroke-width': .8 }), petEllipse(43, 4.4, 1.5, 6.2, pink, { transform: 'rotate(-16 43 10)' }),
      petEllipse(50, 4, 3.4, 9, c.coat, { transform: 'rotate(4 50 10)', stroke: c.shade, 'stroke-width': .8 }), petEllipse(50, 4.4, 1.5, 6.2, pink, { transform: 'rotate(4 50 10)' })],
    hamster: [petEllipse(39.6, 9.6, 4.2, 4.2, c.shade), petEllipse(39.8, 9.8, 2.3, 2.3, pink), petEllipse(55, 8.6, 4, 4, c.shade), petEllipse(55, 8.8, 2.2, 2.2, pink)],
    fox: [petPath('M36.6 14 L36.6 1.5 L46 8.4 Z', c.coat), petPath('M38.4 9.6 L38.2 4.6 L42.4 8 Z', c.ink), petPath('M50 8 L58.4 1.5 L58.8 14 Z', c.coat), petPath('M53.4 7.6 L57 4.6 L57.2 9.6 Z', c.ink)],
  }[kind];
}
function petMarkings(kind, c) {
  return {
    cat: [petStroke('M22 18.6 L23 23 M28 17.4 L28.6 22 M34 18 L34 22', c.shade, 2.4), petStroke('M46 8.6 L46.4 12 M50 8.2 L50 11.4', c.shade, 1.8)],
    dog: [petPath('M19 20 C25 15.6 33 16 37 19 C32 22 24 23 19 20 Z', c.shade, { opacity: .55 })],
    rabbit: [],
    hamster: [petPath('M14 24 C18 15 34 14 39 21 C32 24 22 25 14 24 Z', c.shade, { opacity: .6 })],
    fox: [],
  }[kind];
}
function petFace(kind, c) {
  const muzzle = kind === 'rabbit' ? [] : [petEllipse(54.6, 23.6, kind === 'fox' ? 7 : 6, 4.8, c.belly)];
  const cheeks = kind === 'hamster' ? [petEllipse(47.6, 25, 4.8, 4, c.belly)] : [];
  const whiskers = kind === 'dog' || kind === 'fox' ? [] : [petStroke('M48 24.6 L42 24 M48.4 26.4 L43 27.4', c.ink, .7, { opacity: .5 })];
  return [
    ...muzzle, ...cheeks,
    petEllipse(46.4, 24, 2.6, 1.7, '#ff9aa8', { opacity: .75 }),
    petNode('g', { class: 'pet-eyes-open' }, [petEllipse(49.4, 18.6, 1.9, 2.4, c.ink), petEllipse(50.1, 17.7, .7, .8, '#ffffff'), petEllipse(56.6, 18.6, 1.4, 2.2, c.ink), petEllipse(57, 17.8, .5, .7, '#ffffff')]),
    petNode('g', { class: 'pet-eyes-closed' }, [petStroke('M47.4 18.6 Q49.4 20.8 51.4 18.6 M55.2 18.6 Q56.6 20.4 58 18.6', c.ink, 1.2)]),
    petEllipse(58, 22, 1.8, 1.3, kind === 'rabbit' || kind === 'hamster' ? '#f08c9c' : c.ink),
    petStroke('M57.6 23.4 Q56.6 26 53.6 25.2', c.ink, 1),
    ...whiskers,
    petPath('M54 25.4 Q56.6 25.8 57.2 24.6 Q57.6 28.6 55.6 28.8 Q54 28.6 54 25.4 Z', '#ff7f92', { class: 'pet-tongue' }),
  ];
}
function createPet(provider) {
  const kind = petKindFor(provider), c = petKinds[kind];
  const chubby = kind === 'hamster' ? 1.6 : 0;
  const collar = kind === 'dog' ? [petStroke('M37 27.4 Q40.6 31.4 45 30.6', '#e4572e', 2.4), petEllipse(42.6, 32.2, 1.7, 1.7, '#ffd24a')] : [];
  const zzz = [['M6 4', 64, 8, 4], ['', 69, 2, 5]].map(([, x, y, size], i) => petStroke(`M${x - 4} ${y} h${size} l-${size} ${size} h${size}`, '#8d7bd0', 1.5, { class: 'pet-z pet-z' + i }));
  const svg = petNode('svg', { viewBox: '0 -8 74 54', class: 'pet', 'data-kind': kind, 'aria-hidden': 'true', focusable: 'false' }, [
    petEllipse(34, 44, 24, 1.8, 'currentColor', { class: 'pet-shadow', opacity: .16 }),
    petNode('g', { class: 'pet-bob' }, [
      petTail(kind, c),
      petLeg(20, c.shade, 'pet-leg-b'), petLeg(38, c.shade, 'pet-leg-a'),
      petEllipse(27, 27, 16 + chubby, 10 + chubby, c.coat, kind === 'rabbit' ? { stroke: c.shade, 'stroke-width': .8 } : {}),
      petEllipse(27, 32, 11, 4.4 + chubby / 2, c.belly),
      ...petMarkings(kind, c),
      petLeg(16, c.coat, 'pet-leg-a'), petLeg(34, c.coat, 'pet-leg-b'),
      petNode('g', { class: 'pet-head' }, [
        ...petEars(kind, c),
        petEllipse(48, 19, 12.4, 11.4, c.coat, kind === 'rabbit' ? { stroke: c.shade, 'stroke-width': .8 } : {}),
        ...petFace(kind, c),
      ]),
      ...collar,
    ]),
    petNode('g', { class: 'pet-sleep' }, zzz),
    petPath('M38 2 Q34.6 7 38 8.6 Q41.4 7 38 2 Z', '#5bb8f5', { class: 'pet-sweat' }),
  ]);
  return svg;
}
function updatePet(svg, mood, rate) {
  if (svg.dataset.mood !== mood) svg.dataset.mood = mood;
  const stride = petStrideSeconds(rate) + 's';
  if (svg.style.getPropertyValue('--pet-stride') !== stride) svg.style.setProperty('--pet-stride', stride);
}

// 省電動畫: CSS animations tick at the monitor refresh rate even when nothing moves, so this mode
// turns them off and poses the running animals four times a second instead.
let ecoFrame = 0;
function ecoTick() {
  const root = document.documentElement;
  if (root.dataset.eco !== 'true' || root.dataset.idle === 'true' || window.matchMedia('(prefers-reduced-motion: reduce)').matches) return;
  ecoFrame = (ecoFrame + 1) % 8;
  for (const node of document.querySelectorAll('.habitat[data-act="run"], .pet[data-mood="run"], .run-lane[data-mood="run"]')) {
    node.dataset.pose = String(ecoFrame % 2);
    node.style.setProperty('--frame', String(ecoFrame));
  }
}
if (typeof setInterval === 'function') setInterval(ecoTick, 250);
