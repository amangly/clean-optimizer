export type Tier = "safe" | "risky";
export type ItemKind = "power" | "multi" | "sched" | "check" | "cache";

export type GpuInfo = {
  name: string;
  vendor: string;
  pnp: string;
  discrete: boolean;
};

export type HardwareInfo = {
  cpuName: string;
  cpuCores: number;
  ramGb: number;
  isLaptop: boolean;
  windows: string;
  isAdmin: boolean;
  gpus: GpuInfo[];
  mainGpu: GpuInfo | null;
  displays: string[];
  brand: string;
};

export type ItemView = {
  id: string;
  name: string;
  note: string;
  tier: Tier;
  kind: ItemKind;
  admin: boolean;
  default: boolean;
  bulkSelect: boolean;
  reboot: boolean;
  requiresGame: boolean;
  applicable: boolean;
  optimized: boolean;
  attention: boolean;
  detail: string;
};

export type DetectReport = {
  hardware: HardwareInfo;
  gamePath: string | null;
  items: ItemView[];
  gpuGuide: string[];
  spoofModels: string[];
  recommendedSpoof: string | null;
};

export type ItemResult = {
  id: string;
  name: string;
  ok: boolean;
  changed: boolean;
  skipped: boolean;
  attention: boolean;
  reboot: boolean;
  message: string;
};

export type ApplyReport = {
  applyId: string;
  results: ItemResult[];
  backupFile: string | null;
  succeeded: number;
  failed: number;
  skipped: number;
  attention: number;
};

export type RestoreItem = {
  id: string;
  name: string;
  selective: boolean;
  conflict: boolean;
  detail: string;
};

export type RestoreReport = {
  restored: number;
  failed: number;
  skipped: number;
  notes: string[];
  results: ItemResult[];
};

export type Preset = {
  id: string;
  name: string;
  note: string;
  builtin: boolean;
  items: string[];
};

export type LiveMetrics = {
  cpuPct: number | null;
  gpuPct: number | null;
  ramPct: number | null;
  cpuTemp: number | null;
  gpuTemp: number | null;
  fps: number | null;
  fps1pct: number | null;
  note: string;
};

export type Prefs = {
  telemetry: boolean;
  disclaimerAccepted: boolean;
  theme: string;
};

export type CheckResult = {
  id: string;
  ok: boolean;
  attention: boolean;
  text: string;
};

export type CacheReport = {
  deletedFiles: number;
  bytes: number;
  skipped: number;
  paths: string[];
};

export type Candidate = {
  groupId: string;
  variantId: string;
  displayName: string;
  itemIds: string[];
  itemSetHash: string;
  purpose: string;
};

export type ExperimentState = {
  schemaVersion: number;
  libraryVersion: number;
  experimentId: string;
  status: string;
  sceneId: string;
  currentGroup: string | null;
  baselineRuns: number;
  kept: string[];
  rolledBack: string[];
  gamePath: string | null;
};

export type UpdateInfo = {
  current: string;
  latest: string | null;
  notes: string;
  setupUrl: string | null;
  assetUrl: string | null;
  sha256: string | null;
  available: boolean;
  reached: boolean;
};

export type TabId = "optimize" | "tune" | "fix" | "reference" | "log";
