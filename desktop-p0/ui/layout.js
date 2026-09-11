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
