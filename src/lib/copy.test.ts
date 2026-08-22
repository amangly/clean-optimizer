import { describe, expect, it } from "vitest";
import { banned, copy, flattenCopy } from "./copy";

describe("copy", () => {
  it("has no banned slop words", () => {
    const text = flattenCopy(copy).toLowerCase();
    for (const word of banned) {
      expect(text.includes(word), word).toBe(false);
    }
  });

  it("has no em dash", () => {
    expect(flattenCopy(copy).includes("\u2014")).toBe(false);
  });

  it("names the product", () => {
    expect(copy.appName).toBe("Clean Optimizer");
  });
});
