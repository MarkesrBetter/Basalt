export type ThemeMode =
  | "Dark"
  | "Ember"
  | "Light"
  | "Spruce"
  | "Cobalt"
  | "Nether"
  | "Frost"
  | "Sculk"
  | "End"
  | "Cherry";
export type AccentTone =
  | "Auto"
  | "Copper"
  | "Emerald"
  | "Cyan"
  | "Amber"
  | "Crimson"
  | "Amethyst"
  | "Custom";
export type CornerStyle = "Sharp" | "Balanced" | "Soft";
export type LayoutMode =
  | "Sidebar"
  | "Topbar"
  | "CompactDeck"
  | "RightRail"
  | "MinimalRail"
  | "DeckMode";
export type GraphicsPreset = "Low" | "Medium" | "High" | "Ultra" | "Custom";
export type JvmGcMode = "G1GC" | "ZGC";
export type SkinModel = "Auto" | "Slim" | "Classic";
export type GpuTier = "Discrete" | "Integrated" | "VirtualOrSoftware";
export type LaunchMode = "title" | "singleplayer" | "multiplayer";
export type UiLanguage = "En" | "Tr" | "De";

export interface GraphicsSettings {
  preset: GraphicsPreset;
  render_distance: number;
  simulation_distance: number;
  fov_degrees: number;
  vsync: boolean;
  max_fps: number;
  mouse_sensitivity: number;
  smooth_lighting: boolean;
  fancy_graphics: boolean;
  jvm_ram_mb: number;
  jvm_gc: JvmGcMode;
  show_fps_hud: boolean;
  auto_configured: boolean;
}

export interface SavedServer {
  name: string;
  address: string;
}

export const SUPPORTED_MC_VERSIONS = [
  "1.21.4",
  "1.21.3",
  "1.21.2",
  "1.21.1",
  "1.21",
  "1.20.6",
  "1.20.4",
  "1.20.2",
  "1.20.1",
  "1.20",
  "1.19.4",
  "1.19.2",
  "1.18.2",
  "1.17.1",
  "1.16.5",
] as const;

export const MC_VERSION_PROTOCOLS: Record<string, number> = {
  "1.21.4": 769,
  "1.21.3": 768,
  "1.21.2": 768,
  "1.21.1": 767,
  "1.21": 767,
  "1.20.6": 766,
  "1.20.4": 765,
  "1.20.2": 764,
  "1.20.1": 763,
  "1.20": 763,
  "1.19.4": 762,
  "1.19.2": 760,
  "1.18.2": 758,
  "1.17.1": 756,
  "1.16.5": 754,
};

export interface AppConfig {
  mc_version: string;
  username: string;
  skin_file: string | null;
  skin_model: SkinModel;
  theme: ThemeMode;
  ui_accent: AccentTone;
  custom_accent_hex: string | null;
  ui_corner: CornerStyle;
  ui_layout: LayoutMode;
  ui_compact: boolean;
  ui_reduced_motion: boolean;
  ui_sounds: boolean;
  ui_language: UiLanguage;
  discord_rpc: boolean;
  download_threads: number;
  network_boost: boolean;
  process_boost: boolean;
  last_save_id: string | null;
  saved_servers: SavedServer[];
  graphics: GraphicsSettings;
}

export interface DiscordRpcStatus {
  enabled: boolean;
  connected: boolean;
  pipe_name: string;
  details: string;
  state_label: string;
  started_epoch_secs: number;
  github_url: string;
}

export interface ClientOptimizationReport {
  cdn_ping_ms: number | null;
  local_ping_ms: number | null;
  freed_bytes: number;
  dns_flushed: boolean;
  applied_flags_count: number;
  active_flags: string[];
}

export interface TurboSettingsDto {
  config: AppConfig;
  discord_rpc_status: DiscordRpcStatus;
}

export interface MinecraftSaveSummary {
  id: string;
  modified_epoch_secs: number;
  size_bytes: number;
}

export interface HardwareProfile {
  cpu_name: string;
  physical_cores: number;
  logical_cores: number;
  total_ram_mb: number;
  available_ram_mb: number;
  gpu_name: string;
  gpu_backend: string;
  gpu_device_type: GpuTier;
}

export interface VanillaEnvStatus {
  active_version: string;
  installed_versions: string[];
  vanilla_dir: string;
  runs_dir: string;
  saves_dir: string;
  client_ready: boolean;
  assets_ready: boolean;
  java_ready: boolean;
  java_home: string | null;
  java_version_label: string;
  java_hint: string;
}

export interface VersionChangeDto {
  config: AppConfig;
  vanilla: VanillaEnvStatus;
  discord_rpc_status: DiscordRpcStatus;
}

export interface RunningSessionDto {
  pid: number;
  kind: "launch" | "prepare" | "java-install" | string;
  label: string;
}

export interface MinecraftSoundsDto {
  ready_count: number;
  total_count: number;
  missing_keys: string[];
  sounds: Record<string, string>;
}

export interface BootstrapDto {
  config: AppConfig;
  saves: MinecraftSaveSummary[];
  hardware: HardwareProfile;
  vanilla: VanillaEnvStatus;
  discord_rpc_status: DiscordRpcStatus;
  skin_rgba: number[];
  skin_is_slim: boolean;
  running_session: RunningSessionDto | null;
  hardware_notice: string | null;
}

export interface SkinResultDto {
  rgba: number[];
  path: string | null;
  skin_model: SkinModel;
  skin_is_slim: boolean;
  note: string;
}

export interface SkinModelResultDto {
  skin_model: SkinModel;
  skin_is_slim: boolean;
}

export interface OptimizeResultDto {
  graphics: GraphicsSettings;
  reason: string;
}

export interface ServerPingInfo {
  address: string;
  version_name: string;
  protocol_version: number;
  online_players: number;
  max_players: number;
  description: string;
  latency_ms: number;
}

export type LocalHandshakeResult =
  | {
      kind: "ReadyInPlay";
      data: { server_info: string; keep_alive_tested: boolean };
    }
  | { kind: "RequiresOfficialAuth" }
  | { kind: "Disconnected"; data: string };

export interface LaunchLogPayload {
  line: string;
  is_err: boolean;
}

export interface SessionEndedPayload {
  kind: string;
  exit_code: number | null;
  error: string | null;
  saves: MinecraftSaveSummary[];
  vanilla: VanillaEnvStatus;
}
