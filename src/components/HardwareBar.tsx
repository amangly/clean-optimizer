import { formatRam } from "@/lib/format";
import type { HardwareInfo, LiveMetrics } from "@/lib/types";

type Props = {
  hardware: HardwareInfo;
  metrics: LiveMetrics | null;
};

export function HardwareBar({ hardware, metrics }: Props) {
  const gpu = hardware.mainGpu?.name ?? "No GPU";
  const cpuLoad = metrics?.cpuPct != null ? `${metrics.cpuPct}%` : "n/a";
  const ramLoad = metrics?.ramPct != null ? `${metrics.ramPct}%` : "n/a";
  return (
    <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
      <HwCell label="CPU" value={hardware.cpuName} note={`${hardware.cpuCores} threads · ${cpuLoad}`} />
      <HwCell label="GPU" value={gpu} note={hardware.mainGpu?.vendor ?? ""} />
      <HwCell label="Memory" value={formatRam(hardware.ramGb)} note={`${ramLoad} in use`} />
      <HwCell
        label="System"
        value={hardware.windows}
        note={`${hardware.brand || "PC"} · ${hardware.isLaptop ? "laptop" : "desktop"}`}
      />
    </div>
  );
}

function HwCell({ label, value, note }: { label: string; value: string; note: string }) {
  return (
    <div className="rounded-lg border bg-card px-3 py-2">
      <div className="text-[11px] text-muted-foreground">{label}</div>
      <div className="truncate text-sm font-medium">{value}</div>
      <div className="truncate text-xs text-muted-foreground">{note}</div>
    </div>
  );
}
