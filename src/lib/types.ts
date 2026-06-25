
export type Source = 'winget' | 'scoop' | 'choco';

export const SOURCE_LABELS: Record<Source, string> = {
  winget: 'winget',
  scoop: 'scoop',
  choco: 'choco'
};

export interface Package {
  id: string;
  name: string;
  source: Source;
  version: string | null;
  availableVersion: string | null;
  publisher: string | null;
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

export interface CuratedApp {
  id: string;
  source: Source;
  name: string | null;
  description: string | null;
  homepage: string | null;
  icon: string | null;
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
