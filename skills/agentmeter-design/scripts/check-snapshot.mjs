#!/usr/bin/env node
// Validates a Dashboard Snapshot against the AgentMeter design invariants.
// Dependency-free (Node >= 18). See references/01-domain-model.md and 03-quota-semantics-and-display.md.
//
// Usage:
//   node check-snapshot.mjs <snapshot.json> [--previous <earlier-snapshot.json>] [--providers a,b,c]
// Exit codes: 0 = no errors (warnings allowed), 1 = invariant violations, 2 = bad usage / unreadable input.

import { readFileSync } from 'node:fs';
import { pathToFileURL } from 'node:url';

export const DEFAULT_PROVIDERS = ['codex', 'claude', 'copilot', 'antigravity'];

const ENUMS = {
  availability: ['available', 'not_installed', 'setup_required', 'needs_login', 'unsupported', 'permission_denied'],
  collection_state: ['idle', 'collecting', 'ready', 'backing_off', 'error'],
  freshness: ['unknown', 'fresh', 'stale'],
  collector_maturity: ['stable', 'experimental', 'unsupported'],
  data_quality: ['official', 'local_observed', 'estimated', 'manual'],
};

// Keys that must never appear anywhere in a snapshot (privacy: no credentials, no raw paths).
const FORBIDDEN_KEY = /^(token|access_?token|refresh_?token|secret|password|cookie|authorization|api_?key|csrf|pair_?code|path|serial)$/i;
const WINDOWS_ABSOLUTE_PATH = /(^|[\s"'])[A-Za-z]:\\[^\s"']+/;
const BEARER = /\bbearer\s+[A-Za-z0-9._~+/-]{8,}/i;

const isNumber = value => typeof value === 'number' && Number.isFinite(value);

/** @returns {{errors: string[], warnings: string[]}} */
export function checkSnapshot(snapshot, { previous = null, providers = DEFAULT_PROVIDERS } = {}) {
  const errors = [];
  const warnings = [];
  const err = (where, message) => errors.push(`${where}: ${message}`);
  const warn = (where, message) => warnings.push(`${where}: ${message}`);

  if (snapshot === null || typeof snapshot !== 'object' || Array.isArray(snapshot)) {
    return { errors: ['snapshot: must be a JSON object'], warnings };
  }

  // --- envelope -----------------------------------------------------------
  if (!Number.isInteger(snapshot.schema_version) || snapshot.schema_version < 1) err('schema_version', 'must be an integer >= 1');
  if (typeof snapshot.stream_id !== 'string' || snapshot.stream_id === '') err('stream_id', 'must be a non-empty string unique per app run');
  if (!Number.isInteger(snapshot.revision) || snapshot.revision < 0) err('revision', 'must be a non-negative integer');
  if (typeof snapshot.generated_at !== 'string' || Number.isNaN(Date.parse(snapshot.generated_at))) err('generated_at', 'must be an RFC 3339 timestamp');
  if (typeof snapshot.paused !== 'boolean') err('paused', 'must be a boolean');
  if (!Array.isArray(snapshot.providers)) {
    err('providers', 'must be an array');
    return { errors, warnings };
  }

  // --- provider set -------------------------------------------------------
  const seen = new Map();
  for (const view of snapshot.providers) {
    const id = view?.provider;
    if (typeof id !== 'string') { err('providers[]', 'each entry needs a string "provider"'); continue; }
    if (seen.has(id)) err(`providers.${id}`, 'duplicate provider entry');
    seen.set(id, view);
  }
  for (const id of providers) if (!seen.has(id)) err(`providers.${id}`, 'missing; a snapshot is complete, never partial (ADR 0010)');
  for (const id of seen.keys()) if (!providers.includes(id)) warn(`providers.${id}`, 'not in the expected provider set');

  // --- per provider -------------------------------------------------------
  for (const [id, view] of seen) {
    const at = `providers.${id}`;
    const windows = Array.isArray(view.quota_windows) ? view.quota_windows : null;
    if (windows === null) err(`${at}.quota_windows`, 'must be an array ([] when nothing is collected; never mock values)');
    const hasObservation = (windows?.length ?? 0) > 0 || (Array.isArray(view.source_usage) && view.source_usage.length > 0);

    // Orthogonal axes: validate each independently.
    for (const axis of ['availability', 'collection_state', 'freshness', 'collector_maturity', 'data_quality']) {
      const value = view[axis];
      if (ENUMS[axis].includes(value)) continue;
      // Documented transient: before the first collection, availability/data_quality may be "unknown".
      if (value === 'unknown' && (axis === 'availability' || axis === 'data_quality') && !hasObservation) {
        warn(`${at}.${axis}`, '"unknown" tolerated only until the first collection');
        continue;
      }
      // Documented Copilot special case.
      if (axis === 'freshness' && value === 'provider_reported_period' && id === 'copilot') {
        warn(`${at}.freshness`, '"provider_reported_period" is a known deviation from the freshness enum');
        continue;
      }
      err(`${at}.${axis}`, `"${value}" is not one of ${ENUMS[axis].join(' | ')}`);
    }

    // failure_code explains a failure; it does not replace collection_state.
    if (view.failure_code !== null && view.failure_code !== undefined) {
      if (typeof view.failure_code !== 'string' || view.failure_code === '') err(`${at}.failure_code`, 'must be null or a non-empty string');
      if (!['error', 'backing_off'].includes(view.collection_state)) {
        err(`${at}`, `failure_code "${view.failure_code}" requires collection_state error|backing_off, got "${view.collection_state}"`);
      }
    } else if (view.collection_state === 'error') {
      warn(`${at}`, 'collection_state=error without a failure_code; the user cannot be told why');
    }

    // Freshness needs a timestamp to be about something.
    if ((view.freshness === 'fresh' || view.freshness === 'stale') && view.collected_at == null) {
      err(`${at}.freshness`, `"${view.freshness}" requires collected_at; use "unknown" when nothing was ever collected`);
    }
    if (view.freshness === 'fresh' && view.collection_state === 'error') {
      err(`${at}`, 'freshness "fresh" contradicts collection_state "error"; a failed refresh must mark retained data stale');
    }
    if (snapshot.paused === true && view.collection_state === 'collecting') {
      err(`${at}.collection_state`, 'paused snapshots must not report active collection');
    }

    // Quota windows.
    (windows ?? []).forEach((window, index) => {
      const w = `${at}.quota_windows[${index}]`;
      if (window === null || typeof window !== 'object') { err(w, 'must be an object'); return; }
      const { remaining_percent: remaining, used_percent: used } = window;
      if (remaining !== null && remaining !== undefined) {
        if (!isNumber(remaining)) err(`${w}.remaining_percent`, 'must be a finite number or null (unknown is null, never a string/zero placeholder)');
        else if (remaining < 0 || remaining > 100) err(`${w}.remaining_percent`, `${remaining} outside 0..100; clamp the display value, keep the raw value elsewhere`);
      }
      if (used !== null && used !== undefined && !isNumber(used)) err(`${w}.used_percent`, 'must be a finite number or null');
      if (isNumber(remaining) && isNumber(used) && used <= 100 && Math.abs(remaining + used - 100) > 0.51) {
        err(w, `used_percent ${used} and remaining_percent ${remaining} disagree (must sum to 100)`);
      }
      if (isNumber(remaining) && isNumber(used) && used > 100 && remaining !== 0) {
        err(w, `over limit (used ${used}) must display remaining_percent 0, got ${remaining}`);
      }
      if (window.unit === 'percent' && remaining === undefined && used === undefined) {
        warn(w, 'percent window carries neither remaining_percent nor used_percent');
      }
      if (id === 'copilot' && window.resets_at != null) {
        err(`${w}.resets_at`, 'Copilot reset time is not trustworthy and must be null');
      }
    });
  }

  // --- privacy: scan the whole document ----------------------------------
  const scan = (value, path) => {
    if (Array.isArray(value)) { value.forEach((item, i) => scan(item, `${path}[${i}]`)); return; }
    if (value !== null && typeof value === 'object') {
      for (const [key, child] of Object.entries(value)) {
        if (FORBIDDEN_KEY.test(key)) err(`${path}.${key}`, 'forbidden key: snapshots must not carry credentials, pair material, serials or paths');
        scan(child, `${path}.${key}`);
      }
      return;
    }
    if (typeof value === 'string') {
      if (WINDOWS_ABSOLUTE_PATH.test(value)) err(path, 'contains an absolute Windows path');
      if (BEARER.test(value)) err(path, 'contains what looks like a bearer token');
      if (/[^\s@]+@[^\s@]+\.[A-Za-z]{2,}/.test(value) && !/^[^\s@]\*{3}@/.test(value)) err(path, 'contains an unmasked e-mail address');
    }
  };
  scan(snapshot, '$');

  // --- stream continuity --------------------------------------------------
  if (previous && typeof previous === 'object') {
    if (previous.stream_id === snapshot.stream_id) {
      if (Number.isInteger(previous.revision) && Number.isInteger(snapshot.revision)) {
        if (snapshot.revision < previous.revision) err('revision', `went backwards within stream (${previous.revision} -> ${snapshot.revision})`);
        if (snapshot.revision === previous.revision && JSON.stringify(snapshot.providers) !== JSON.stringify(previous.providers)) {
          err('revision', 'content changed but revision did not advance (heartbeats may not bump revision; real changes must)');
        }
        if (snapshot.revision > previous.revision && JSON.stringify(snapshot.providers) === JSON.stringify(previous.providers)) {
          warn('revision', 'revision advanced with identical provider content; unchanged publishes should not bump it');
        }
      }
    } else {
      warn('stream_id', 'new stream (app restart): revisions are not comparable across streams');
    }
  }

  return { errors, warnings };
}

function main(argv) {
  const args = argv.slice(2);
  const take = flag => {
    const index = args.indexOf(flag);
    if (index < 0) return null;
    const [, value] = args.splice(index, 2);
    return value ?? null;
  };
  const previousPath = take('--previous');
  const providerList = take('--providers');
  const file = args[0];
  if (!file) {
    console.error('usage: check-snapshot.mjs <snapshot.json> [--previous <file>] [--providers a,b,c]');
    return 2;
  }
  let snapshot, previous = null;
  try {
    snapshot = JSON.parse(readFileSync(file, 'utf8'));
    if (previousPath) previous = JSON.parse(readFileSync(previousPath, 'utf8'));
  } catch (error) {
    console.error(`cannot read input: ${error.message}`);
    return 2;
  }
  const providers = providerList ? providerList.split(',').map(s => s.trim()).filter(Boolean) : DEFAULT_PROVIDERS;
  const { errors, warnings } = checkSnapshot(snapshot, { previous, providers });
  for (const w of warnings) console.log(`warn   ${w}`);
  for (const e of errors) console.log(`ERROR  ${e}`);
  console.log(errors.length ? `FAIL: ${errors.length} error(s), ${warnings.length} warning(s)` : `OK: 0 errors, ${warnings.length} warning(s)`);
  return errors.length ? 1 : 0;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) process.exitCode = main(process.argv);
