import { describe, expect, it } from "vitest";
import { reachedEnd } from "./disclaimer-scroll";

describe("reachedEnd", () => {
  it("opens when content fits the viewport", () => {
    expect(reachedEnd(0, 400, 320)).toBe(true);
  });

  it("stays closed until the last 4 pixels", () => {
    expect(reachedEnd(0, 200, 400)).toBe(false);
    expect(reachedEnd(196, 200, 400)).toBe(true);
  });
});
