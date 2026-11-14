import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  AccentTone,
  AppConfig,
  BootstrapDto,
  CornerStyle,
  GraphicsSettings,
  LaunchLogPayload,
  LaunchMode,
  LocalHandshakeResult,
  MinecraftSaveSummary,
  MinecraftSoundsDto,
  OptimizeResultDto,
  GradleEnvStatus,
  LayoutMode,
  RunningSessionDto,
  SavedServer,
  ServerPingInfo,
  SessionEndedPayload,
  SkinModel,
  SkinModelResultDto,
  SkinResultDto,
  ThemeMode,
} from "./types";

export function isTauriRuntime(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

function makeSampleSkinRgba(): number[] {
  const out = new Array<number>(64 * 64 * 4).fill(0);
  const paintRect = (
    rx: number,
    ry: number,
    rw: number,
    rh: number,
    r: number,
    g: number,
    b: number
  ) => {
    for (let y = ry; y < ry + rh; y += 1) {
      for (let x = rx; x < rx + rw; x += 1) {
        const idx = (y * 64 + x) * 4;
        out[idx] = r;
        out[idx + 1] = g;
        out[idx + 2] = b;
        out[idx + 3] = 255;
      }
    }
  };
  paintRect(0, 0, 32, 16, 214, 165, 126);
  paintRect(8, 8, 8, 8, 214, 165, 126);
  paintRect(8, 8, 8, 2, 54, 42, 34);
  paintRect(9, 11, 2, 1, 28, 36, 48);
  paintRect(13, 11, 2, 1, 28, 36, 48);
  paintRect(11, 13, 2, 1, 174, 120, 92);
  paintRect(16, 16, 24, 16, 46, 70, 94);
  paintRect(20, 20, 8, 2, 62, 92, 120);
  paintRect(40, 16, 16, 16, 68, 102, 130);
  paintRect(44, 28, 4, 4, 206, 156, 118);
  paintRect(32, 48, 16, 16, 68, 102, 130);
  paintRect(36, 60, 4, 4, 206, 156, 118);
  paintRect(0, 16, 16, 16, 48, 54, 64);
  paintRect(16, 48, 16, 16, 42, 48, 58);
  return out;
}

// Isolated browser-preview sample data (only used outside Tauri desktop runtime)
const previewState: BootstrapDto = {
  config: {
    username: "Oyuncu",
    skin_file: null,
    skin_model: "Auto",
    theme: "Dark",
    ui_accent: "Auto",
    custom_accent_hex: "#a47cff",
    ui_corner: "Balanced",
    ui_layout: "Sidebar",
    ui_compact: false,
    ui_reduced_motion: false,
    ui_sounds: false,
    last_save_id: "Hayatta Kalma (Ornek)",
    saved_servers: [
      {
        name: "Yerel Test Sunucusu",
        address: "127.0.0.1:25565",
      },
    ],
    graphics: {
      preset: "Ultra",
      render_distance: 16,
      simulation_distance: 12,
      fov_degrees: 75,
      vsync: true,
      max_fps: 240,
      mouse_sensitivity: 0.5,
      smooth_lighting: true,
      fancy_graphics: true,
      jvm_ram_mb: 4096,
      jvm_gc: "ZGC",
      show_fps_hud: true,
      auto_configured: true,
    },
  },
  saves: [
    {
      id: "Hayatta Kalma (Ornek)",
      modified_epoch_secs: 1_763_120_000,
      size_bytes: 2_450_000,
    },
  ],
  hardware: {
    cpu_name: "Örnek Çok Çekirdekli İşlemci",
    physical_cores: 6,
    logical_cores: 12,
    total_ram_mb: 16384,
    available_ram_mb: 9800,
    gpu_name: "Örnek Harici GPU",
    gpu_backend: "Dx12",
    gpu_device_type: "Discrete",
  },
  gradle: {
    gradle_dir: "C:\\basalt\\client-gradle",
    runs_dir: "C:\\basalt\\client-gradle\\run",
    saves_dir: "C:\\basalt\\client-gradle\\run\\saves",
    gradlew_ready: true,
    mod_jar_built: true,
    java_ready: true,
    java_home: "C:\\Program Files\\Eclipse Adoptium\\jdk-21",
    java_version_label: "Java 21.0.11",
    java_hint: "Sistemde uyumlu Java 21+ JDK kurulumu algılandı.",
  },
  skin_rgba: makeSampleSkinRgba(),
  skin_is_slim: true,
  running_session: null,
  hardware_notice: null,
};

async function loadBrowserPreviewDefaultSkinRgba(): Promise<number[] | null> {
  if (typeof Image === "undefined" || typeof document === "undefined") {
    return null;
  }
  return new Promise((resolve) => {
    const img = new Image();
    img.onload = () => {
      const canvas = document.createElement("canvas");
      canvas.width = 64;
      canvas.height = 64;
      const ctx = canvas.getContext("2d");
      if (!ctx) {
        resolve(null);
        return;
      }
      ctx.drawImage(img, 0, 0, 64, 64);
      const data = ctx.getImageData(0, 0, 64, 64).data;
      resolve(Array.from(data));
    };
    img.onerror = () => resolve(null);
    img.src = "/default_skin.png";
  });
}

export async function getBootstrapState(): Promise<BootstrapDto> {
  if (isTauriRuntime()) {
    return invoke<BootstrapDto>("get_bootstrap_state");
  }
  const realDefault = await loadBrowserPreviewDefaultSkinRgba();
  if (realDefault) {
    previewState.skin_rgba = realDefault;
  }
  const snapshot = structuredClone(previewState);
  if (new URLSearchParams(window.location.search).get("java") === "missing") {
    snapshot.gradle.java_ready = false;
    snapshot.gradle.java_home = null;
    snapshot.gradle.java_version_label = "Java 21 JDK bulunamadı";
    snapshot.gradle.java_hint =
      "Bu bilgisayarda Java 21 JDK algılanmadı. Yönetici izni gerektirmeden Otomatik Java 21 Kur ile yükleyebilirsiniz.";
  }
  return snapshot;
}

export async function listSaves(): Promise<MinecraftSaveSummary[]> {
  if (isTauriRuntime()) {
    return invoke<MinecraftSaveSummary[]>("list_saves");
  }
  return structuredClone(previewState.saves);
}

export async function selectSave(saveId: string | null): Promise<void> {
  if (isTauriRuntime()) {
    await invoke<void>("select_save", { saveId });
    return;
  }
  previewState.config.last_save_id = saveId;
}

export async function deleteSave(saveId: string): Promise<MinecraftSaveSummary[]> {
  if (isTauriRuntime()) {
    return invoke<MinecraftSaveSummary[]>("delete_save", { saveId });
  }
  previewState.saves = previewState.saves.filter((s) => s.id !== saveId);
  if (previewState.config.last_save_id === saveId) {
    previewState.config.last_save_id = previewState.saves[0]?.id ?? null;
  }
  return structuredClone(previewState.saves);
}

export async function saveProfileSettings(input: {
  username: string;
  theme: ThemeMode;
  ui_accent: AccentTone;
  custom_accent_hex: string | null;
  ui_corner: CornerStyle;
  ui_layout: LayoutMode;
  ui_compact: boolean;
  ui_reduced_motion: boolean;
  ui_sounds: boolean;
}): Promise<AppConfig> {
  if (isTauriRuntime()) {
    return invoke<AppConfig>("save_profile_settings", { input });
  }
  const clean = input.username.trim();
  if (!clean || !/^[A-Za-z0-9_-]{1,16}$/.test(clean)) {
    throw new Error("Kullanıcı adı 1-16 karakter arası harf, rakam, _ veya - olmalıdır.");
  }
  previewState.config.username = clean;
  previewState.config.theme = input.theme;
  previewState.config.ui_accent = input.ui_accent;
  previewState.config.custom_accent_hex = input.custom_accent_hex;
  previewState.config.ui_corner = input.ui_corner;
  previewState.config.ui_layout = input.ui_layout;
  previewState.config.ui_compact = input.ui_compact;
  previewState.config.ui_reduced_motion = input.ui_reduced_motion;
  previewState.config.ui_sounds = input.ui_sounds;
  return structuredClone(previewState.config);
}

export async function saveGraphicsSettings(
  graphics: GraphicsSettings
): Promise<AppConfig> {
  if (isTauriRuntime()) {
    return invoke<AppConfig>("save_graphics_settings", { graphics });
  }
  previewState.config.graphics = structuredClone(graphics);
  return structuredClone(previewState.config);
}

export async function autoOptimizeGraphics(): Promise<OptimizeResultDto> {
  if (isTauriRuntime()) {
    return invoke<OptimizeResultDto>("auto_optimize_graphics");
  }
  const gfx: GraphicsSettings = {
    ...previewState.config.graphics,
    preset: "Ultra",
    render_distance: 16,
    simulation_distance: 12,
    jvm_ram_mb: 4096,
    jvm_gc: "ZGC",
    max_fps: 240,
    auto_configured: true,
  };
  previewState.config.graphics = gfx;
  return {
    graphics: gfx,
    reason: "Örnek profil: 16 chunk görüş mesafesi, 4096 MB JVM belleği ve ZGC seçildi (önizleme).",
  };
}

export async function pickAndImportSkin(): Promise<SkinResultDto | null> {
  if (isTauriRuntime()) {
    return invoke<SkinResultDto | null>("pick_and_import_skin");
  }
  throw new Error("Yerel dosya seçimi yalnızca masaüstü uygulamasında (basalt.exe) çalışır.");
}

export async function resetSkin(): Promise<SkinResultDto> {
  if (isTauriRuntime()) {
    return invoke<SkinResultDto>("reset_skin");
  }
  previewState.config.skin_file = null;
  previewState.config.skin_model = "Auto";
  previewState.skin_is_slim = true;
  previewState.skin_rgba = (await loadBrowserPreviewDefaultSkinRgba()) ?? makeSampleSkinRgba();
  return {
    rgba: previewState.skin_rgba,
    path: null,
    skin_model: "Auto",
    skin_is_slim: true,
    note: "Varsayılan Basalt ince (Slim) skini geri yüklendi (tarayıcı önizlemesi).",
  };
}

export async function saveSkinModel(skinModel: SkinModel): Promise<SkinModelResultDto> {
  if (isTauriRuntime()) {
    return invoke<SkinModelResultDto>("save_skin_model", { skinModel });
  }
  previewState.config.skin_model = skinModel;
  const resolvedSlim =
    skinModel === "Slim" ? true : skinModel === "Classic" ? false : true;
  previewState.skin_is_slim = resolvedSlim;
  return {
    skin_model: skinModel,
    skin_is_slim: resolvedSlim,
  };
}

export async function addServer(
  name: string,
  address: string
): Promise<SavedServer[]> {
  if (isTauriRuntime()) {
    return invoke<SavedServer[]>("add_server", { name, address });
  }
  const addr = address.trim();
  if (!addr) throw new Error("Sunucu adresi boş olamaz.");
  previewState.config.saved_servers.push({
    name: name.trim() || "Yerel Sunucu",
    address: addr,
  });
  return structuredClone(previewState.config.saved_servers);
}

export async function removeServer(index: number): Promise<SavedServer[]> {
  if (isTauriRuntime()) {
    return invoke<SavedServer[]>("remove_server", { index });
  }
  previewState.config.saved_servers.splice(index, 1);
  return structuredClone(previewState.config.saved_servers);
}

export async function pingServerCmd(address: string): Promise<ServerPingInfo> {
  if (isTauriRuntime()) {
    return invoke<ServerPingInfo>("ping_server_cmd", { address });
  }
  throw new Error(
    `Tarayıcı önizlemesinde yerel TCP SLP sorgusu yapılamaz (${address}). Masaüstü uygulamasını kullanın.`
  );
}

export async function probeLocalServerCmd(
  address: string,
  username: string
): Promise<LocalHandshakeResult> {
  if (isTauriRuntime()) {
    return invoke<LocalHandshakeResult>("probe_local_server_cmd", {
      address,
      username,
    });
  }
  throw new Error(
    `Tarayıcı önizlemesinde yerel TCP el sıkışma testi yapılamaz (${address}). Masaüstü uygulamasını kullanın.`
  );
}

export async function prepareGradleClient(): Promise<RunningSessionDto> {
  if (isTauriRuntime()) {
    return invoke<RunningSessionDto>("prepare_gradle_client");
  }
  throw new Error(
    "Gradle Fabric 1.21.4 ön derlemesi yalnızca masaüstü uygulamasında (basalt.exe) çalıştırılabilir."
  );
}

export async function installJava21(): Promise<RunningSessionDto> {
  if (isTauriRuntime()) {
    return invoke<RunningSessionDto>("install_java_21");
  }
  throw new Error(
    "Otomatik Java 21 kurulumu yalnızca masaüstü uygulamasında (basalt.exe) çalıştırılabilir."
  );
}

export async function recheckJava21(): Promise<GradleEnvStatus> {
  if (isTauriRuntime()) {
    return invoke<GradleEnvStatus>("recheck_java_21");
  }
  return structuredClone(previewState.gradle);
}

export async function openJavaDownloadPage(): Promise<void> {
  if (isTauriRuntime()) {
    await invoke<void>("open_java_download_page");
    return;
  }
  window.open(
    "https://adoptium.net/temurin/releases/?version=21&os=windows&arch=x64&package=jdk",
    "_blank",
    "noopener"
  );
}

export async function setFullscreen(fullscreen: boolean): Promise<void> {
  if (isTauriRuntime()) {
    await invoke<void>("set_fullscreen", { fullscreen });
    return;
  }
  if (typeof document === "undefined") return;
  try {
    if (fullscreen && !document.fullscreenElement) {
      await document.documentElement.requestFullscreen?.();
    } else if (!fullscreen && document.fullscreenElement) {
      await document.exitFullscreen?.();
    }
  } catch {
    // Headless or browser gesture constraint
  }
}

export async function windowMinimize(): Promise<void> {
  if (isTauriRuntime()) {
    await invoke<void>("window_minimize");
  }
}

export async function windowToggleMaximize(): Promise<boolean> {
  if (isTauriRuntime()) {
    return invoke<boolean>("window_toggle_maximize");
  }
  if (typeof document === "undefined") return false;
  try {
    if (document.fullscreenElement) {
      await document.exitFullscreen?.();
      return false;
    }
    await document.documentElement.requestFullscreen?.();
    return Boolean(document.fullscreenElement);
  } catch {
    return false;
  }
}

export async function windowIsMaximized(): Promise<boolean> {
  if (isTauriRuntime()) {
    return invoke<boolean>("window_is_maximized");
  }
  return Boolean(typeof document !== "undefined" && document.fullscreenElement);
}

export async function windowClose(): Promise<void> {
  if (isTauriRuntime()) {
    await invoke<void>("window_close");
    return;
  }
  window.close();
}

export async function windowStartDrag(): Promise<void> {
  if (isTauriRuntime()) {
    await invoke<void>("window_start_drag");
  }
}

export async function launchClient(input: {
  mode: LaunchMode;
  save_id: string | null;
  server_address: string | null;
}): Promise<RunningSessionDto> {
  if (isTauriRuntime()) {
    return invoke<RunningSessionDto>("launch_client", { input });
  }
  throw new Error(
    "Gradle Minecraft 1.21.4 istemcisi yalnızca masaüstü uygulamasında (basalt.exe) başlatılabilir."
  );
}

export async function onLaunchLog(
  cb: (payload: LaunchLogPayload) => void
): Promise<() => void> {
  if (isTauriRuntime()) {
    return listen<LaunchLogPayload>("basalt://launch-log", (event) => {
      cb(event.payload);
    });
  }
  return () => {};
}

export async function loadMinecraftSounds(): Promise<MinecraftSoundsDto> {
  if (isTauriRuntime()) {
    return invoke<MinecraftSoundsDto>("load_minecraft_sounds");
  }
  return { ready_count: 0, total_count: 8, missing_keys: [], sounds: {} };
}

export async function onMinecraftSoundsReady(
  cb: () => void
): Promise<() => void> {
  if (isTauriRuntime()) {
    return listen<number>("basalt://minecraft-sounds-ready", () => {
      cb();
    });
  }
  return () => {};
}

export async function onSessionEnded(
  cb: (payload: SessionEndedPayload) => void
): Promise<() => void> {
  if (isTauriRuntime()) {
    return listen<SessionEndedPayload>("basalt://session-ended", (event) => {
      cb(event.payload);
    });
  }
  return () => {};
}
