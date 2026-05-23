import { describe, it, expect } from "vitest";
import { parseAppError } from "./errors";

describe("parseAppError", () => {
  it("normalizes a Tauri AppError object {kind, message}", () => {
    const result = parseAppError({ kind: "PermissionDenied", message: "operation cancelled" });
    expect(result).toEqual({ kind: "PermissionDenied", message: "operation cancelled" });
  });

  it("normalizes a plain string", () => {
    const result = parseAppError("something went wrong");
    expect(result).toEqual({ kind: "Unknown", message: "something went wrong" });
  });

  it("normalizes a native Error", () => {
    const result = parseAppError(new Error("network failure"));
    expect(result).toEqual({ kind: "Error", message: "network failure" });
  });

  it("normalizes an unknown/unexpected value", () => {
    const result = parseAppError(42);
    expect(result.kind).toBe("Unknown");
    expect(typeof result.message).toBe("string");
  });

  it("uses message field from AppError object when present", () => {
    const result = parseAppError({ kind: "Backend", message: "dpkg error" });
    expect(result.kind).toBe("Backend");
    expect(result.message).toBe("dpkg error");
  });
});
