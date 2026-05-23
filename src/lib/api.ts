import { invoke, convertFileSrc } from "@tauri-apps/api/core";
import type { App, AppList } from "./types";

export const listApps = (): Promise<AppList> => invoke<AppList>("list_apps");

export const getAppDetails = (uid: string): Promise<string | null> =>
  invoke<string | null>("get_app_details", { uid });

export const iconSrc = (app: App): string | null =>
  app.icon_path ? convertFileSrc(app.icon_path) : null;

export const uninstallApp = (uid: string): Promise<void> =>
  invoke<void>("uninstall_app", { uid });

export const launchApp = (uid: string): Promise<void> =>
  invoke<void>("launch_app", { uid });
