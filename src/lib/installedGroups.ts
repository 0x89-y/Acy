// Bucketing for the installed library - used by the home screen's Library rail
// (buckets as categories) and its installed-apps pane.
import type { Package, Source } from '$lib/types';

export type BucketKey = Source | 'games' | 'ms-system' | 'other';

// The left-rail selection in Library mode: All apps, a bucket, or Updates.
export type LibSelection = 'all' | BucketKey | 'updates';

// winget surfaces lots of entries that aren't from a real package manager:
// Add/Remove-Programs (`ARP\…`) and MSIX/AppX (`MSIX\…`) packages. We only
// reclassify THOSE into noise buckets - real managed apps (a proper winget/
// scoop/choco/msstore id) always stay in their manager group, so e.g. the
// "Epic Games Launcher" managed by winget isn't yanked into Games.
//
// Detection is best-effort (name/id heuristics) and easy to tune.
const GAMES =
  /(\\steam app\b|steam app|epic games|\bgog\b|gog galaxy|gog\.com|\bea app\b|ea desktop|origin games|ubisoft|uplay|battle\.net|blizzard|battlefield|\briot\b|riot games|riot client|valorant|league of legends|hoyoplay|genshin|honkai|zenless zone|vintage story)/i;
const MS_SYSTEM =
  /(visual c\+\+|redistributable|webview2|windows sdk|windows software development kit|\.net\s+(runtime|sdk|host|desktop runtime|targeting pack)|microsoft edge update|windows app runtime)/i;
// Microsoft's MSIX publisher hash - first-party Store/built-in packages.
const MS_MSIX_PUBLISHER = '_8wekyb3d8bbwe';
// Reliable signals from the ARP registry (publisher / install path), which
// catch launcher-installed games whose name carries no marker (e.g. Diablo).
const GAME_PUBLISHERS =
  /(valve|blizzard|ubisoft|electronic arts|rockstar|bethesda|riot games|cd projekt|epic games|\bgog\b|square enix|activision|\bsega\b|capcom|bandai namco|2k games|xbox game studios|mojang|devolver|paradox interactive|larian|mihoyo|hoyoverse|cognosphere|anego studios)/i;
const GAME_PATHS =
  /(steamapps|[\\/]steam[\\/]|epic games|gog galaxy|gog games|[\\/]gog[\\/]|ubisoft|uplay|riot games|battle\.net|[\\/]ea games[\\/]|origin games|[\\/]games[\\/])/i;

export function bucketKey(p: Package): BucketKey {
  if (p.source !== 'winget') return p.source;
  const idl = p.id.toLowerCase();
  const isArp = idl.startsWith('arp\\');
  const isMsix = idl.startsWith('msix\\');
  if (!isArp && !isMsix) return 'winget'; // a real winget-managed app
  const hay = `${p.name} ${p.id}`;
  const pub = (p.publisher ?? '').toLowerCase();
  const loc = (p.installLocation ?? '').toLowerCase();
  if (GAMES.test(hay) || GAME_PUBLISHERS.test(pub) || GAME_PATHS.test(loc)) return 'games';
  if (MS_SYSTEM.test(p.name)) return 'ms-system';
  if (isMsix && idl.includes(MS_MSIX_PUBLISHER)) return 'ms-system';
  return 'other';
}

// Extract a Steam appid from a winget ARP id like
// `ARP\Machine\X64\Steam App 1621690` (the trailing `\`-segment). Mirrors the
// Rust `steam_appid` in src-tauri/src/icons.rs, which pulls box art from Steam's
// CDN for these; the UI uses it to render that art edge-to-edge (cover).
export function steamAppId(id: string): string | null {
  const last = (id.split('\\').pop() ?? id).trim();
  const m = /^steam app (\d+)$/i.exec(last);
  return m ? m[1] : null;
}

// `managed` buckets (a real package manager) drive the cross-manager dupe note.
export type Bucket = {
  key: BucketKey;
  label: string;
  /** The manager badge to draw, if this bucket is a real package manager. */
  badge: Source | null;
  managed: boolean;
};

export const BUCKETS: Bucket[] = [
  { key: 'winget', label: 'winget', badge: 'winget', managed: true },
  { key: 'scoop', label: 'Scoop', badge: 'scoop', managed: true },
  { key: 'choco', label: 'Chocolatey', badge: 'choco', managed: true },
  { key: 'msstore', label: 'Microsoft Store', badge: 'msstore', managed: true },
  { key: 'games', label: 'Games', badge: null, managed: false },
  { key: 'ms-system', label: 'Windows components', badge: null, managed: false },
  { key: 'other', label: 'Other apps', badge: null, managed: false }
];

export const BUCKET_BY_KEY = new Map(BUCKETS.map((b) => [b.key, b] as const));
