import { describe, expect, it } from "vitest";
import { USAGE_LIMIT, pushUsageSample } from "./usage-samples";

describe("usage samples", () => {
  it("appends and treats missing load as zero", () => {
    const one = pushUsageSample([], null, 40);
    expect(one).toEqual([{ i: 0, cpu: 0, ram: 40 }]);
    expect(pushUsageSample(one, 12, 41)).toEqual([
      { i: 0, cpu: 0, ram: 40 },
      { i: 1, cpu: 12, ram: 41 },
    ]);
  });

  it("keeps a fixed window", () => {
    let samples = pushUsageSample([], 1, 1);
    for (let n = 0; n < USAGE_LIMIT + 4; n += 1) {
      samples = pushUsageSample(samples, n, n);
    }
    expect(samples).toHaveLength(USAGE_LIMIT);
    expect(samples[0]?.i).toBe(5);
    expect(samples[USAGE_LIMIT - 1]?.cpu).toBe(USAGE_LIMIT + 3);
  });
});
