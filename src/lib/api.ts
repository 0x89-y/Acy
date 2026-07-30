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

export const updateCuratedCatalog = (apply: boolean) =>
  invoke<{
    updated: boolean;
    available: boolean;
    version: number;
    appCount: number;
    message: string;
  }>('update_curated_catalog', { apply });

export interface CustomCatalogInfo {
  source: string;
  isUrl: boolean;
  version: number;
  appCount: number;
}

export const customCatalogInfo = () =>
  invoke<CustomCatalogInfo | null>('custom_catalog_info');

export const setCustomCatalog = (source: string, isUrl: boolean) =>
  invoke<CustomCatalogInfo>('set_custom_catalog', { source, isUrl });

export const clearCustomCatalog = () => invoke<void>('clear_custom_catalog');

export const pickCatalogFile = () => invoke<string | null>('pick_catalog_file');

export const search = (query: string, sources: Source[]) =>
  invoke<SearchHit[]>('search', { query, sources });

export const listInstalled = (sources: Source[]) =>
  invoke<Package[]>('list_installed', { sources });

export const listInstalledFast = () => invoke<Package[]>('list_installed_fast');

export const listUpdates = (sources: Source[]) =>
  invoke<Package[]>('list_updates', { sources });

export const appInfo = (source: Source, id: string) =>
  invoke<Package | null>('app_info', { source, id });

export const install = (source: Source, id: string, opId: string) =>
  invoke<number>('install', { source, id, opId });

export const scoopNeededBucket = (id: string) =>
  invoke<string | null>('scoop_needed_bucket', { id });

export const refetchMissingIcons = (
  items: { source: Source; id: string; homepage: string | null; gameName?: string | null }[],
  steamGridKey?: string | null
) =>
  invoke<{ fetched: number; failed: number }>('refetch_missing_icons', {
    items,
    steamgridKey: steamGridKey || null
  });

export const appIconState = (source: Source, id: string) =>
  invoke<{ cached: boolean; deleted: boolean }>('app_icon_state', { source, id });

export const refreshAppIcon = (
  source: Source,
  id: string,
  homepage?: string | null,
  steamGridKey?: string | null,
  gameName?: string | null
) =>
  invoke<string | null>('refresh_app_icon', {
    source,
    id,
    homepage: homepage ?? null,
    steamgridKey: steamGridKey || null,
    gameName: gameName || null
  });

export const deleteAppIcon = (source: Source, id: string) =>
  invoke<void>('delete_app_icon', { source, id });

export const uninstall = (source: Source, id: string, opId: string) =>
  invoke<number>('uninstall', { source, id, opId });

export const upgrade = (source: Source, id: string, opId: string) =>
  invoke<number>('upgrade', { source, id, opId });

export const bootstrapManager = (source: Source, opId: string) =>
  invoke<number>('bootstrap_manager', { source, opId });

export const setUpdateCount = (count: number) => invoke<void>('set_update_count', { count });

export const notify = (title: string, body: string) => invoke<void>('notify', { title, body });

export const pickInstaller = () => invoke<string | null>('pick_installer');

export const scoopBuckets = () => invoke<string[]>('scoop_buckets');

export const scoopKnownBuckets = () => invoke<string[]>('scoop_known_buckets');

export const addScoopBucket = (name: string, opId: string) =>
  invoke<number>('add_scoop_bucket', { name, opId });

export const removeScoopBucket = (name: string, opId: string) =>
  invoke<number>('remove_scoop_bucket', { name, opId });

export const wingetUpdateSources = (opId: string) =>
  invoke<number>('winget_update_sources', { opId });

export const scoopUpdate = (opId: string) => invoke<number>('scoop_update', { opId });

export const scoopCleanup = (opId: string) => invoke<number>('scoop_cleanup', { opId });

export function onOpLog(cb: (line: LogLine) => void): Promise<UnlistenFn> {
  return listen<LogLine>('op-log', (event) => cb(event.payload));
}

export function onIconRefetchProgress(
  cb: (p: { current: number; total: number }) => void
): Promise<UnlistenFn> {
  return listen<{ current: number; total: number }>('icon-refetch-progress', (e) =>
    cb(e.payload)
  );
}
