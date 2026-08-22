export function formatRam(gb: number): string {
  if (!Number.isFinite(gb) || gb <= 0) {
    return "RAM unknown";
  }
  return `${gb.toFixed(1)} GB RAM`;
}

export function formatBytes(bytes: number): string {
  if (bytes < 1024) {
    return `${bytes} B`;
  }
  if (bytes < 1024 * 1024) {
    return `${(bytes / 1024).toFixed(1)} KB`;
  }
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

export function statusLabel(optimized: boolean, kind: string): string {
  if (kind === "check") {
    return "Check";
  }
  if (kind === "cache") {
    return "Cache";
  }
  return optimized ? "On" : "Off";
}
