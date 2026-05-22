import { describe, it, expect } from "vitest";
import { humanSize, humanDate } from "./format";

describe("humanSize", () => {
  it("returns placeholder for null", () => {
    expect(humanSize(null)).toBe("—");
  });

  it("formats zero bytes", () => {
    expect(humanSize(0)).toBe("0 B");
  });

  it("formats sub-kilobyte values in bytes", () => {
    expect(humanSize(512)).toBe("512 B");
  });

  it("formats kilobytes (1024-based)", () => {
    expect(humanSize(1024)).toBe("1 KB");
  });

  it("formats megabytes with one decimal", () => {
    // 97086620 / 1024^2 = 92.589... -> 92.6 MB
    expect(humanSize(97086620)).toBe("92.6 MB");
  });

  it("formats gigabytes", () => {
    expect(humanSize(3 * 1024 * 1024 * 1024)).toBe("3 GB");
  });

  it("returns placeholder for negative values", () => {
    expect(humanSize(-1)).toBe("—");
  });
});

describe("humanDate", () => {
  it("returns placeholder for null", () => {
    expect(humanDate(null)).toBe("—");
  });

  it("returns placeholder for invalid date strings", () => {
    expect(humanDate("not-a-date")).toBe("—");
  });

  it("formats a valid RFC3339 date", () => {
    const out = humanDate("2026-05-23T00:00:00Z");
    expect(out).not.toBe("—");
    expect(out).toMatch(/2026/);
  });
});
