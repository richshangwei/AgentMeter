// Only the exchange CSRF value is script-readable. The actual Device Pair remains HttpOnly.
function createPairRecovery(storage) {
  const key = 'agentmeter.pair-exchange.v1';
  const valid = value => typeof value === 'string' && /^[0-9a-f]{32}$/.test(value);
  return {
    read() { try { const value = storage.getItem(key); return valid(value) ? value : ''; } catch { return ''; } },
    save(value) { if (!valid(value)) return false; try { storage.setItem(key,value); return true; } catch { return false; } },
    clear() { try { storage.removeItem(key); } catch { /* denied storage never grants authorization */ } }
  };
}
