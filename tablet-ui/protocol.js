function validProviderId(value) {
  return typeof value === 'string' && /^[a-z0-9][a-z0-9._-]{0,79}$/.test(value) && value !== 'constructor' && value !== '__proto__';
}

function createSnapshotGate() {
  let stream = null, revision = -1;
  return (value, allowNewStream = false) => {
    const names = Array.isArray(value?.providers) ? value.providers.map(item => item?.provider) : [];
    if (!value || typeof value.stream_id !== 'string' || !value.stream_id ||
        !Number.isSafeInteger(value.revision) || value.revision < 0 ||
        !Array.isArray(value.providers) || !value.providers.length ||
        names.some(name => !validProviderId(name)) ||
        new Set(names).size !== names.length) {
      throw Error('invalid complete snapshot');
    }
    if (value.stream_id !== stream) {
      if (!allowNewStream) return false;
      stream = value.stream_id; revision = -1;
    }
    if (value.revision <= revision) return false;
    revision = value.revision;
    return true;
  };
}

function createEventParser(onSnapshot, onEvent) {
  let pending = '';
  return chunk => {
    pending += chunk;
    let match;
    while ((match = /\r?\n\r?\n/.exec(pending))) {
      if (match.index > 262144) throw Error('event too large');
      const raw = pending.slice(0, match.index);
      pending = pending.slice(match.index + match[0].length);
      onEvent();
      const lines = raw.split(/\r?\n/);
      if (lines.includes('event: dashboard')) {
        const data = lines.filter(line => line.startsWith('data:')).map(line => line.slice(5).replace(/^ /, '')).join('\n');
        onSnapshot(JSON.parse(data));
      }
    }
    if (pending.length > 262144) throw Error('event too large');
  };
}

function createWatchdog(expire, schedule = setTimeout, cancel = clearTimeout) {
  let timer;
  return {
    touch() { cancel(timer); timer = schedule(expire, 45000); },
    stop() { cancel(timer); timer = undefined; }
  };
}
