// Mirrors the serialized shapes from the Rust core (all camelCase).

/** Real package managers (searchable, listable, detectable). */
export type Manager = 'winget' | 'scoop' | 'choco' | 'msstore';

/** Any install source, including the file-based "local" one. */
export type Source = Manager | 'local';

export interface Package {
  id: string;
  name: string;
  source: Source;
  version: string | null;
  availableVersion: string | null;
  publisher: string | null;
  /** Filesystem install path (ARP entries only), used to categorize games. */
  installLocation: string | null;
  homepage: string | null;
  description: string | null;
  installed: boolean;
}

export interface SearchHit {
  name: string;
  publisher: string | null;
  description: string | null;
  installed: boolean;
  variants: Package[];
}

export interface ManagerStatus {
  source: Source;
  available: boolean;
  needsSetup: boolean;
  detail: string | null;
}

/** One installable option for an app: a manager plus its package id. */
export interface Variant {
  source: Source;
  id: string;
}

export interface CuratedApp {
  id: string;
  source: Source;
  name: string | null;
  description: string | null;
  homepage: string | null;
  icon: string | null;
  /** Other managers the same app is available from (besides the primary source/id). */
  alternates: Variant[];
  /** Free-form labels (open source, free, chromium, a license, ...) for display + filtering. */
  tags: string[];
  /** Optional donation / support URL. */
  donate: string | null;
  /** Optional link to the app's release notes / changelog. */
  releaseNotes: string | null;
  /** True when the user added this app; built-in apps come from the bundled catalog. */
  custom: boolean;
}

export interface CuratedCategory {
  id: string;
  title: string;
  apps: CuratedApp[];
}

export interface CuratedFile {
  version: number;
  categories: CuratedCategory[];
}

export interface LogLine {
  opId: string;
  stream: 'stdout' | 'stderr';
  line: string;
}
