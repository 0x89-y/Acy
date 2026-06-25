import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type {
  CuratedFile,
  LogLine,
  ManagerStatus,
  Package,
  SearchHit,
  Source
} from './types';

export const detectManagers = () => invoke<ManagerStatus[]>('detect_managers');

export const getCurated = () => invoke<CuratedFile>('get_curated');

export const search = (query: string, sources: Source[]) =>
  invoke<SearchHit[]>('search', { query, sources });

export const listInstalled = (sources: Source[]) =>
  invoke<Package[]>('list_installed', { sources });

export const listUpdates = (sources: Source[]) =>
  invoke<Package[]>('list_updates', { sources });

export const appInfo = (source: Source, id: string) =>
  invoke<Package | null>('app_info', { source, id });

export const install = (source: Source, id: string, opId: string) =>
  invoke<number>('install', { source, id, opId });

export const uninstall = (source: Source, id: string, opId: string) =>
  invoke<number>('uninstall', { source, id, opId });

export const upgrade = (source: Source, id: string, opId: string) =>
  invoke<number>('upgrade', { source, id, opId });

export const upgradeAll = (source: Source, opId: string) =>
  invoke<number>('upgrade_all', { source, opId });

export const bootstrapManager = (source: Source, opId: string) =>
  invoke<number>('bootstrap_manager', { source, opId });

export function onOpLog(cb: (line: LogLine) => void): Promise<UnlistenFn> {
  return listen<LogLine>('op-log', (event) => cb(event.payload));
}
