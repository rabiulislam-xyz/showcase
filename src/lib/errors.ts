/**
 * Normalize any value thrown by a Tauri invoke into a structured error shape.
 *
 * Tauri serializes AppError as `{kind: string, message: string}` when the
 * command returns Err(). This helper handles that shape, plain strings, native
 * Error objects, and any unknown value so callers never need to inspect the raw
 * thrown value themselves.
 */
export function parseAppError(e: unknown): { kind: string; message: string } {
  if (typeof e === "object" && e !== null) {
    const obj = e as Record<string, unknown>;
    if (typeof obj["kind"] === "string") {
      return {
        kind: obj["kind"],
        message: typeof obj["message"] === "string" ? obj["message"] : String(e),
      };
    }
    if (e instanceof Error) {
      return { kind: "Error", message: e.message };
    }
  }
  if (typeof e === "string") {
    return { kind: "Unknown", message: e };
  }
  return { kind: "Unknown", message: String(e) };
}
