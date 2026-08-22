export type UsageSample = {
  i: number;
  cpu: number;
  ram: number;
};

export const USAGE_LIMIT = 30;

export function pushUsageSample(
  prev: UsageSample[],
  cpu: number | null,
  ram: number | null,
): UsageSample[] {
  const next = [
    ...prev,
    {
      i: (prev.at(-1)?.i ?? -1) + 1,
      cpu: cpu ?? 0,
      ram: ram ?? 0,
    },
  ];
  return next.length > USAGE_LIMIT ? next.slice(-USAGE_LIMIT) : next;
}
