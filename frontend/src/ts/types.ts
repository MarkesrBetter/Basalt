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

export interface AppConfig {
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
  last_save_id: string | null;
  saved_servers: SavedServer[];
  graphics: GraphicsSettings;
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

export interface GradleEnvStatus {
  gradle_dir: string;
  runs_dir: string;
  saves_dir: string;
  gradlew_ready: boolean;
  mod_jar_built: boolean;
  java_ready: boolean;
  java_home: string | null;
  java_version_label: string;
  java_hint: string;
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
  gradle: GradleEnvStatus;
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
  gradle: GradleEnvStatus;
}
