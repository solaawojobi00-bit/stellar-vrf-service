import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname } from 'node:path';

interface OracleState {
  lastProcessedLedger: number;
}

export function loadState(path: string): OracleState {
  if (!existsSync(path)) {
    return { lastProcessedLedger: 0 };
  }
  return JSON.parse(readFileSync(path, 'utf8'));
}

export function saveState(path: string, state: OracleState): void {
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, JSON.stringify(state, null, 2));
}
