export type Source = "apt" | "flatpak" | "snap";

export interface App {
  uid: string;
  source: Source;
  name: string;
  summary: string | null;
  description: string | null;
  version: string | null;
  icon_path: string | null;
  size_bytes: number | null;
  install_date: string | null;
  publisher: string | null;
  categories: string[];
  exec: string | null;
  pkg_ref: string;
  removable: boolean;
  protected_reason: string | null;
}

export interface AppList {
  apps: App[];
  warnings: string[];
}
