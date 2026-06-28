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

export const saveCurated = (file: CuratedFile) => invoke<void>('save_curated', { file });

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

export const setUpdateCount = (count: number) => invoke<void>('set_update_count', { count });

export const notify = (title: string, body: string) => invoke<void>('notify', { title, body });

export const pickInstaller = () => invoke<string | null>('pick_installer');

export const scoopBuckets = () => invoke<string[]>('scoop_buckets');

export const scoopKnownBuckets = () => invoke<string[]>('scoop_known_buckets');

export const addScoopBucket = (name: string, opId: string) =>
  invoke<number>('add_scoop_bucket', { name, opId });

export const wingetUpdateSources = (opId: string) =>
  invoke<number>('winget_update_sources', { opId });

export const scoopUpdate = (opId: string) => invoke<number>('scoop_update', { opId });

export const scoopCleanup = (opId: string) => invoke<number>('scoop_cleanup', { opId });

export function onOpLog(cb: (line: LogLine) => void): Promise<UnlistenFn> {
  return listen<LogLine>('op-log', (event) => cb(event.payload));
}
