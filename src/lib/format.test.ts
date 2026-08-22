import { describe, expect, it } from "vitest";
import { formatBytes, formatRam, statusLabel } from "./format";

describe("format", () => {
  it("formats ram", () => {
    expect(formatRam(32)).toBe("32 GB");
    expect(formatRam(0)).toBe("RAM unknown");
  });

  it("formats bytes", () => {
    expect(formatBytes(32)).toBe("32 B");
    expect(formatBytes(2048)).toBe("2.0 KB");
  });

  it("labels item state", () => {
    expect(statusLabel(true, "multi")).toBe("On");
    expect(statusLabel(false, "check")).toBe("Check");
  });
});
