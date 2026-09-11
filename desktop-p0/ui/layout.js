const desktopMonitorKey = 'agentmeter.desktop-monitor-selection.v1';

function normalizeDesktopSelection(raw, available) {
  const ids = [...new Set(available.filter(id => typeof id === 'string' && id))];
  if (!Array.isArray(raw)) return ids;
  return [...new Set(raw)].filter(id => ids.includes(id));
}

function desktopViewportGrid(_width, _height, selectedCount = 0) {
  const count = Math.max(0,Math.min(4,Number(selectedCount) || 0));
  if (count <= 1) return {columns:1,rows:1,pageSize:4};
  if (count === 2) return {columns:2,rows:1,pageSize:4};
  if (count === 3) return {columns:1,rows:3,pageSize:4};
  return {columns:2,rows:2,pageSize:4};
}

function pagedMonitorIds(selected, page, pageSize) {
  const pages = Math.max(1,Math.ceil(selected.length / pageSize));
  const safePage = Math.max(0,Math.min(Number(page) || 0,pages - 1));
  return {page:safePage,pages,ids:selected.slice(safePage * pageSize,(safePage + 1) * pageSize)};
}

function moveDesktopMonitor(selected, provider, direction) {
  const values=Array.isArray(selected)?[...selected]:[];
  const index=values.indexOf(provider),target=index+(Number(direction)<0?-1:1);
  if(index<0||target<0||target>=values.length)return values;
  [values[index],values[target]]=[values[target],values[index]];
  return values;
}

function quotaPeriodLabel(item) {
  const minutes=Number(item?.window_duration_mins);
  if(Number.isFinite(minutes)&&minutes>0){
    if(minutes===10080)return '每週使用上限';
    if(minutes===300)return '5 小時用量限制';
    return Number.isInteger(minutes/60)?`${minutes/60} 小時用量限制`:`${minutes} 分鐘用量限制`;
  }
  if(item?.window==='five_hour')return '5 小時用量限制';
  if(item?.window==='seven_day')return '每週使用上限';
  return '用量限制';
}

function quotaLimitLabel(item) {
  const id=item?.limit_id||'';
  const name=typeof item?.limit_name==='string'?item.limit_name.trim():'';
  if(id==='codex'||id==='default')return '常規使用額度';
  if(id==='base_model_inference'||name.toLowerCase()==='gpt-reserve')return '備用模型額度';
  if(name)return name;
  if(id==='codex_bengalfox')return 'GPT-5.3-Codex-Spark';
  if(id.startsWith('codex_'))return '模型使用額度';
  return '';
}

function quotaLabel(item) {
  const group=quotaLimitLabel(item);
  if(group)return `${group} · ${quotaPeriodLabel(item)}`;
  const raw=(item?.label||item?.bucket_key||'額度')
    .replace('premium_interactions','Premium interactions')
    .replace('five_hour','5 小時').replace('seven_day','每週')
    .replace('Gemini Models','Gemini').replace('Claude and GPT models','Claude / GPT')
    .replace('Weekly Limit Remaining','每週').replace('Five Hour Limit Remaining','5 小時')
    .replace(/([^·]) (?=(?:每週|5 小時)$)/,'$1 · ');
  const legacy=/^(?:default|codex|base_model_inference|codex_bengalfox) (?:primary|secondary)$/.exec(raw);
  if(!legacy)return raw.replace(/\b(?:primary|secondary)\b/gi,'用量限制').trim();
  const legacyGroup=raw.startsWith('base_model_inference')?'備用模型額度':raw.startsWith('codex_bengalfox')?'GPT-5.3-Codex-Spark':'常規使用額度';
  return `${legacyGroup} · 用量限制`;
}

// ---- Fit-to-viewport sizing (no scrollbars) ----
// Cards are classified by their real rectangle. Wide, short cards lay out their
// heading | quota | actions horizontally; everything else stacks vertically.
const desktopCardMinimum = {normal:{width:290,height:210},wide:{width:520,height:112}};
function desktopCardShape(width, height) {
  return width / Math.max(1,height) >= 2.6 ? 'wide' : 'normal';
}
function desktopCardDensity(width, height, shape = desktopCardShape(width,height)) {
  if (shape === 'wide') return height < 170 ? 'tight' : 'regular';
  if (width < 360 || height < 280) return 'tight';
  if (width >= 620 && height >= 440) return 'roomy';
  return 'regular';
}
// How many cards one page can show while every card — and the densest card's
// quota tiles — stays readable. The count-driven topology (1×1, 2×1, 1×3, 2×2)
// is kept; only small windows fall back to fewer cards per page (pager).
const desktopTileMinimum = {width:90,height:56};
function estimatedQuotaRegion(width, height, shape) {
  return shape === 'wide'
    ? {width:width - 24 - Math.max(130,width * .19) - Math.max(128,width * .2) - 40,height:height - 24}
    : {width:width - 24,height:height - 24 - 44 - 36 - 16};
}
function desktopPageCapacity(areaWidth, areaHeight, selectedCount, gap = 12, densestTiles = 1) {
  const count = Math.max(1,Math.min(4,Math.floor(Number(selectedCount) || 1)));
  if (!(areaWidth > 0 && areaHeight > 0)) return 4;
  for (const size of [...new Set([count,Math.min(count,2),1])]) {
    const grid = desktopViewportGrid(areaWidth,areaHeight,size);
    const width = (areaWidth - gap * (grid.columns - 1)) / grid.columns;
    const height = (areaHeight - gap * (grid.rows - 1)) / grid.rows;
    const shape = desktopCardShape(width,height), minimum = desktopCardMinimum[shape];
    if (width < minimum.width || height < minimum.height) continue;
    const region = estimatedQuotaRegion(width,height,shape);
    const tiles = quotaTileLayout(region.width,region.height,Math.max(1,Number(densestTiles) || 1));
    if (size > 1 && (!tiles || tiles.tileWidth < desktopTileMinimum.width || tiles.tileHeight < desktopTileMinimum.height)) continue;
    return size === count ? 4 : size;
  }
  return 1;
}
const clampNumber = (value, min, max) => Math.max(min,Math.min(max,value));
// Places `count` quota tiles into a width × height region without overflow and
// returns the tile variant plus pixel sizes that make the tile content fit.
function quotaTileLayout(width, height, count) {
  const n = Math.max(1,Math.floor(Number(count) || 1));
  if (!(width > 0 && height > 0)) return null;
  const gap = height < 120 || width < 260 ? 5 : 8;
  let best = null;
  for (let columns = 1; columns <= n; columns++) {
    const rows = Math.ceil(n / columns);
    if (columns > 1 && (columns - 1) * rows >= n) continue;
    const tileWidth = (width - gap * (columns - 1)) / columns;
    const tileHeight = (height - gap * (rows - 1)) / rows;
    if (tileWidth <= 0 || tileHeight <= 0) continue;
    const score = Math.min(tileWidth / 250, tileHeight / 112) * (1 - .03 * (columns * rows - n));
    if (!best || score > best.score + 1e-9) best = {columns,rows,tileWidth,tileHeight,score};
  }
  if (!best) return null;
  const tileWidth = Math.floor(Math.min(best.tileWidth,560)), tileHeight = Math.floor(Math.min(best.tileHeight,260));
  // Every size below is derived from the tile rectangle so the content height
  // (line boxes + gaps + padding) is known to fit before the variant is chosen.
  let variant, pad = 10, label, value, small, ring = 0, resetLines = 1, labelLines = 1, showUsed = true;
  const ringGap = size => clampNumber(size * .12,8,18);
  const ringSize = clampNumber(Math.min(tileHeight - pad * 2,tileWidth * .42),56,190);
  const ringLabel = clampNumber(tileHeight * .12,11,16), ringSmall = clampNumber(tileHeight * .1,10,13);
  const stackLabel = clampNumber(tileWidth * .08,11,15), stackSmall = clampNumber(stackLabel - 1.5,10,13);
  const stackRing = Math.min(tileWidth - pad * 2,tileHeight - pad * 2 - (stackLabel * 1.3 + stackSmall * 2.6 + 12),190);
  if (tileHeight >= 92 && tileHeight - pad * 2 >= 56 && tileWidth - pad * 2 - ringSize - ringGap(ringSize) >= 120) {
    variant = 'ring'; ring = ringSize; label = ringLabel; small = ringSmall; value = ring * .17;
    const available = tileHeight - pad * 2;
    const needed = (labels, smalls) => label * 1.3 * labels + 6 + small * 1.3 * smalls + 4 * 3;
    showUsed = needed(1,2) <= available;
    if (needed(2,showUsed ? 2 : 1) <= available) labelLines = 2;
    if (needed(labelLines,showUsed ? 3 : 2) <= available) resetLines = 2;
  } else if (tileHeight >= 150 && stackRing >= 56) {
    variant = 'stack'; ring = stackRing; label = stackLabel; small = stackSmall; value = ring * .17;
  } else if (tileHeight >= 56) {
    variant = 'bar'; pad = tileHeight < 64 ? 5 : 8;
    label = clampNumber(tileHeight * .2,10.5,14); value = clampNumber(tileHeight * .26,13,24); small = clampNumber(tileHeight * .13,10,12);
    const needed = label * 1.3 + value * 1.15 + small * 1.3 * 2 + 6 + pad * 2;
    showUsed = needed <= tileHeight;
    if (!showUsed) value = clampNumber(tileHeight - pad * 2 - label * 1.3 - small * 1.3 - 4,13,24) / 1.15;
  } else {
    variant = 'line'; pad = tileHeight < 40 ? 2 : 4;
    label = clampNumber(tileHeight * .24,10,13); value = clampNumber(tileHeight * .28,11,16); small = 10; showUsed = false;
  }
  // Bars beside the number need room; otherwise the percentage alone is shown.
  const inlineTrack = variant === 'ring' || variant === 'stack' || tileWidth - pad * 2 - value * 3.8 - 10 >= 18;
  const round = number => Math.round(number * 10) / 10;
  return {columns:best.columns,rows:best.rows,gap,tileWidth,tileHeight,variant,resetLines,labelLines,showUsed,inlineTrack,
    pad,ring:round(ring),label:round(label),value:round(value),small:round(small)};
}
