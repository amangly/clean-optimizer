import { useEffect, useState } from "react";
import { Area, AreaChart, YAxis } from "recharts";
import { ChartContainer, type ChartConfig } from "@/components/ui/chart";
import { formatRam } from "@/lib/format";
import { pushUsageSample, type UsageSample } from "@/lib/usage-samples";
import type { HardwareInfo, LiveMetrics } from "@/lib/types";

const chartConfig = {
  cpu: { label: "CPU", color: "var(--foreground)" },
  ram: { label: "RAM", color: "var(--muted-foreground)" },
} satisfies ChartConfig;

type Props = {
  hardware: HardwareInfo;
  metrics: LiveMetrics | null;
};

function Spark({
  label,
  value,
  dataKey,
  data,
}: {
  label: string;
  value: number | null;
  dataKey: "cpu" | "ram";
  data: UsageSample[];
}) {
  return (
    <div className="flex items-center gap-2">
      <span className="shrink-0 whitespace-nowrap text-[11px] text-muted-foreground tabular-nums">
        {label} {value == null ? "n/a" : `${value}%`}
      </span>
      <ChartContainer
        config={chartConfig}
        className="aspect-auto h-7 w-20 shrink-0 overflow-hidden"
        initialDimension={{ width: 80, height: 28 }}
      >
        <AreaChart data={data} margin={{ top: 2, right: 0, bottom: 2, left: 0 }}>
          <YAxis hide domain={[0, 100]} />
          <Area
            dataKey={dataKey}
            type="monotone"
            fill={`var(--color-${dataKey})`}
            fillOpacity={0.18}
            stroke={`var(--color-${dataKey})`}
            strokeWidth={1.25}
            isAnimationActive={false}
            dot={false}
          />
        </AreaChart>
      </ChartContainer>
    </div>
  );
}

export function HardwareBar({ hardware, metrics }: Props) {
  const [samples, setSamples] = useState<UsageSample[]>([]);

  useEffect(() => {
    if (!metrics) {
      return;
    }
    setSamples((prev) => pushUsageSample(prev, metrics.cpuPct, metrics.ramPct));
  }, [metrics]);

  const title = [
    hardware.cpuName,
    formatRam(hardware.ramGb),
    hardware.mainGpu?.name ?? "No GPU",
    hardware.isLaptop ? "laptop" : "desktop",
  ].join(" · ");

  return (
    <div className="flex shrink-0 items-center gap-4" title={title}>
      <Spark label="CPU" value={metrics?.cpuPct ?? null} dataKey="cpu" data={samples} />
      <Spark label="RAM" value={metrics?.ramPct ?? null} dataKey="ram" data={samples} />
    </div>
  );
}
