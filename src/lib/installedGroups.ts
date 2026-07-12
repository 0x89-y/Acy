import type { Package, Source } from '$lib/types';
import type { InstalledShow } from '$lib/stores/settings';

export type BucketKey = Source | 'games' | 'ms-system' | 'other';

export type LibSelection = 'all' | BucketKey | 'updates';

const GAMES =
  /(\\steam app\b|steam app|epic games|\bgog\b|gog galaxy|gog\.com|\bea app\b|ea desktop|origin games|ubisoft|uplay|battle\.net|blizzard|battlefield|\briot\b|riot games|riot client|valorant|league of legends|hoyoplay|genshin|honkai|zenless zone|vintage story)/i;
const MS_SYSTEM =
  /(visual c\+\+|redistributable|webview2|windows sdk|windows software development kit|\.net\s+(runtime|sdk|host|desktop runtime|targeting pack)|microsoft edge update|windows app runtime)/i;
const MS_MSIX_PUBLISHER = '_8wekyb3d8bbwe';
const GAME_PUBLISHERS =
  /(valve|blizzard|ubisoft|electronic arts|rockstar|bethesda|riot games|cd projekt|epic games|\bgog\b|square enix|activision|\bsega\b|capcom|bandai namco|2k games|xbox game studios|mojang|devolver|paradox interactive|larian|mihoyo|hoyoverse|cognosphere|anego studios)/i;
const GAME_PATHS =
  /(steamapps|[\\/]steam[\\/]|epic games|gog galaxy|gog games|[\\/]gog[\\/]|ubisoft|uplay|riot games|battle\.net|[\\/]ea games[\\/]|origin games|[\\/]games[\\/])/i;

export function bucketKey(p: Package): BucketKey {
  if (p.source !== 'winget') return p.source;
  const idl = p.id.toLowerCase();
  const isArp = idl.startsWith('arp\\');
  const isMsix = idl.startsWith('msix\\');
  if (!isArp && !isMsix) return 'winget';
  const hay = `${p.name} ${p.id}`;
  const pub = (p.publisher ?? '').toLowerCase();
  const loc = (p.installLocation ?? '').toLowerCase();
  if (GAMES.test(hay) || GAME_PUBLISHERS.test(pub) || GAME_PATHS.test(loc)) return 'games';
  if (MS_SYSTEM.test(p.name)) return 'ms-system';
  if (isMsix && idl.includes(MS_MSIX_PUBLISHER)) return 'ms-system';
  return 'other';
}

export type Bucket = {
  key: BucketKey;
  label: string;
  badge: Source | null;
  managed: boolean;
  system: boolean;
};

export const BUCKETS: Bucket[] = [
  { key: 'winget', label: 'winget', badge: 'winget', managed: true, system: false },
  { key: 'scoop', label: 'Scoop', badge: 'scoop', managed: true, system: false },
  { key: 'choco', label: 'Chocolatey', badge: 'choco', managed: true, system: false },
  { key: 'msstore', label: 'Microsoft Store', badge: 'msstore', managed: true, system: false },
  { key: 'games', label: 'Games', badge: null, managed: false, system: false },
  { key: 'ms-system', label: 'Windows components', badge: null, managed: false, system: true },
  { key: 'other', label: 'Other apps', badge: null, managed: false, system: true }
];

export const BUCKET_BY_KEY = new Map(BUCKETS.map((b) => [b.key, b] as const));

export function bucketVisible(key: BucketKey, show: InstalledShow): boolean {
  const b = BUCKET_BY_KEY.get(key);
  if (!b) return false;
  if (show === 'managed') return b.managed;
  if (show === 'hide-system') return !b.system;
  return true;
}
