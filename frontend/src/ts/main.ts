import { applyDynamicDesignTokens, normalizeHex } from "./colors";
import {
  ACCENT_LABELS,
  CORNER_LABELS,
  GPU_TIER_LABELS,
  LANG_INTL_LOCALE,
  LAYOUT_SHORT_LABELS,
  THEME_SHORT_LABELS,
  UI_LANGUAGE_ORDER,
  applyStaticI18n,
  tr,
} from "./i18n";
import * as ipc from "./ipc";
import {
  drawSkinPreview,
  drawTopbarHead,
  renderClientStratumSvg,
  renderDeckCardThumbSvg,
  type SkinViewMode,
} from "./skin_canvas";
import { initMinecraftSounds, playMcSound, setSoundsEnabled, unlockAudioContext } from "./sounds";
import type {
  AccentTone,
  BootstrapDto,
  CornerStyle,
  VanillaEnvStatus,
  GraphicsSettings,
  LaunchMode,
  LayoutMode,
  MinecraftSaveSummary,
  SavedServer,
  ServerPingInfo,
  SkinModel,
  ThemeMode,
  UiLanguage,
} from "./types";

type ViewId = "play" | "worlds" | "profile" | "system";
const VIEW_ORDER: readonly ViewId[] = ["play", "worlds", "profile", "system"];

interface ServerCheckState {
  status: "idle" | "checking" | "online" | "error";
  ping?: ServerPingInfo;
  message?: string;
}

const VALID_USERNAME_RE = /^[A-Za-z0-9_-]{1,16}$/;
const MAX_LOG_LINES = 400;

const THEME_ORDER: readonly ThemeMode[] = [
  "Dark",
  "Ember",
  "Light",
  "Spruce",
  "Cobalt",
  "Nether",
  "Frost",
  "Sculk",
  "End",
  "Cherry",
];
const THEME_ATTRS: Record<ThemeMode, string> = {
  Dark: "dark",
  Ember: "ember",
  Light: "light",
  Spruce: "spruce",
  Cobalt: "cobalt",
  Nether: "nether",
  Frost: "frost",
  Sculk: "sculk",
  End: "end",
  Cherry: "cherry",
};

const ACCENT_ORDER: readonly AccentTone[] = [
  "Auto",
  "Copper",
  "Emerald",
  "Cyan",
  "Amber",
  "Crimson",
  "Amethyst",
  "Custom",
];
const CORNER_ORDER: readonly CornerStyle[] = ["Sharp", "Balanced", "Soft"];

const DESKTOP_LAYOUT_ORDER: readonly LayoutMode[] = [
  "Sidebar",
  "Topbar",
  "CompactDeck",
  "RightRail",
  "MinimalRail",
];
const LAYOUT_ORDER: readonly LayoutMode[] = [
  ...DESKTOP_LAYOUT_ORDER,
  "DeckMode",
];
const LAYOUT_ATTRS: Record<LayoutMode, string> = {
  Sidebar: "sidebar",
  Topbar: "topbar",
  CompactDeck: "compact-deck",
  RightRail: "right-rail",
  MinimalRail: "minimal-rail",
  DeckMode: "deck",
};

type DeckCardKind = "launch" | "world" | "server" | "nav";

interface DeckCard {
  id: string;
  kind: DeckCardKind;
  kicker: string;
  title: string;
  meta: string;
  sub: string;
  tag: string;
  seedKey: string;
  primaryLabel: string;
  secondaryLabel: string;
  secondaryDisabled?: boolean;
  launchMode?: LaunchMode;
  saveId?: string | null;
  serverAddress?: string | null;
  targetView?: ViewId;
}

let state: BootstrapDto | null = null;
let skinViewMode: SkinViewMode = "front";
let pendingDeleteSaveId: string | null = null;
let lastDialogTrigger: HTMLElement | null = null;
let toastTimer: number | null = null;
let logLines: string[] = [];
const serverChecks = new Map<string, ServerCheckState>();

let consoleModeActive = false;
let consoleAudioMuted = false;
let lastDesktopLayout: LayoutMode = "Sidebar";
let deckCards: DeckCard[] = [];
let selectedDeckIdx = 0;
let deckClockTimer: number | null = null;

function byId<T extends HTMLElement>(id: string): T {
  const node = document.getElementById(id);
  if (!node) {
    throw new Error(`Missing UI element: #${id}`);
  }
  return node as T;
}

function getEffectiveLanguage(): UiLanguage {
  const raw = new URLSearchParams(window.location.search).get("lang")?.toLowerCase();
  if (raw === "tr") return "Tr";
  if (raw === "de") return "De";
  if (raw === "en") return "En";
  const cfg = state?.config.ui_language;
  return cfg === "Tr" || cfg === "De" || cfg === "En" ? cfg : "En";
}

function errorMessage(err: unknown): string {
  if (err instanceof Error) return err.message;
  if (typeof err === "string") return err;
  return tr(getEffectiveLanguage(), "err.unexpected");
}

function showToast(message: string, tone: "neutral" | "success" | "error" = "neutral"): void {
  const toast = byId<HTMLDivElement>("toast");
  toast.textContent = message;
  if (tone === "neutral") {
    toast.removeAttribute("data-tone");
  } else {
    toast.setAttribute("data-tone", tone);
  }
  toast.hidden = false;

  if (toastTimer !== null) {
    window.clearTimeout(toastTimer);
  }
  toastTimer = window.setTimeout(() => {
    toast.hidden = true;
    toastTimer = null;
  }, 4200);
}

function setBanner(
  element: HTMLElement,
  message: string | null,
  tone: "neutral" | "success" | "error" = "neutral"
): void {
  if (!message) {
    element.hidden = true;
    element.textContent = "";
    element.removeAttribute("data-tone");
    return;
  }
  element.textContent = message;
  if (tone === "neutral") {
    element.removeAttribute("data-tone");
  } else {
    element.setAttribute("data-tone", tone);
  }
  element.hidden = false;
}

function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes <= 0) return "0 KB";
  if (bytes < 1024 * 1024) {
    return `${Math.max(1, Math.round(bytes / 1024))} KB`;
  }
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function formatEpoch(secs: number): string {
  const lang = getEffectiveLanguage();
  if (!Number.isFinite(secs) || secs <= 0) return tr(lang, "misc.unknownDate");
  try {
    return new Intl.DateTimeFormat(LANG_INTL_LOCALE[lang], {
      dateStyle: "short",
      timeStyle: "short",
    }).format(new Date(secs * 1000));
  } catch {
    return new Date(secs * 1000).toISOString().slice(0, 16).replace("T", " ");
  }
}

function validateUsername(raw: string): string | null {
  const lang = getEffectiveLanguage();
  const trimmed = raw.trim();
  if (!trimmed) {
    return tr(lang, "val.usernameEmpty");
  }
  if (!VALID_USERNAME_RE.test(trimmed)) {
    return tr(lang, "val.usernameChars");
  }
  return null;
}

function setSelectNumericValue(select: HTMLSelectElement, targetValue: number): void {
  const lang = getEffectiveLanguage();
  const strVal = String(targetValue);
  for (const existing of Array.from(select.querySelectorAll('option[data-synthetic="true"]'))) {
    existing.remove();
  }
  let exists = false;
  for (const option of Array.from(select.options)) {
    if (option.value === strVal) {
      exists = true;
      break;
    }
  }
  if (!exists) {
    const opt = document.createElement("option");
    opt.value = strVal;
    opt.dataset.synthetic = "true";
    opt.textContent = `${targetValue} MB (${tr(lang, "system.ramRecommended", { mb: targetValue }).replace(`${targetValue} MB `, "").replace(/^\(|\)$/g, "")})`;
    select.appendChild(opt);
  }
  select.value = strVal;
}

function getEffectiveTheme(): ThemeMode {
  const queryTheme = new URLSearchParams(window.location.search).get("theme")?.toLowerCase();
  const matched = THEME_ORDER.find((t) => t.toLowerCase() === queryTheme);
  return matched ?? state?.config.theme ?? "Dark";
}

function getEffectiveAccent(): AccentTone {
  const raw = new URLSearchParams(window.location.search).get("accent")?.toLowerCase();
  const matched = ACCENT_ORDER.find((item) => item.toLowerCase() === raw);
  return matched ?? state?.config.ui_accent ?? "Auto";
}

function getEffectiveCustomHex(): string {
  const raw = new URLSearchParams(window.location.search).get("customHex");
  return normalizeHex(raw) ?? normalizeHex(state?.config.custom_accent_hex) ?? "#a47cff";
}

function getEffectiveCorner(): CornerStyle {
  const raw = new URLSearchParams(window.location.search).get("corner")?.toLowerCase();
  const matched = CORNER_ORDER.find((item) => item.toLowerCase() === raw);
  return matched ?? state?.config.ui_corner ?? "Balanced";
}

function getEffectiveLayout(): LayoutMode {
  const queryLayout = new URLSearchParams(window.location.search).get("layout")?.toLowerCase();
  if (queryLayout === "topbar") return "Topbar";
  if (queryLayout === "compact-deck" || queryLayout === "compact") return "CompactDeck";
  if (queryLayout === "right-rail" || queryLayout === "right") return "RightRail";
  if (queryLayout === "minimal-rail" || queryLayout === "minimal" || queryLayout === "dock") return "MinimalRail";
  if (queryLayout === "deck" || queryLayout === "deck-mode" || queryLayout === "steamdeck") return "DeckMode";
  if (queryLayout === "sidebar") return "Sidebar";
  if (consoleModeActive) return "DeckMode";
  const cfgLayout = state?.config.ui_layout ?? "Sidebar";
  return cfgLayout === "DeckMode" ? lastDesktopLayout : cfgLayout;
}

function getEffectiveSounds(): boolean {
  const raw = new URLSearchParams(window.location.search).get("sounds")?.toLowerCase();
  if (raw === "0" || raw === "false") return false;
  if (raw === "1" || raw === "true") return true;
  if (getEffectiveLayout() === "DeckMode") {
    return !consoleAudioMuted;
  }
  return state?.config.ui_sounds ?? false;
}

function updateDeckClock(): void {
  const el = document.getElementById("deck-clock");
  if (!el) return;
  const now = new Date();
  el.textContent = now.toLocaleTimeString(LANG_INTL_LOCALE[getEffectiveLanguage()], {
    hour: "2-digit",
    minute: "2-digit",
  });
}

function syncDeckClock(active: boolean): void {
  if (active) {
    updateDeckClock();
    if (deckClockTimer === null) {
      deckClockTimer = window.setInterval(updateDeckClock, 15_000);
    }
  } else if (deckClockTimer !== null) {
    window.clearInterval(deckClockTimer);
    deckClockTimer = null;
  }
}

function currentActiveViewId(): ViewId {
  const activeBtn = document.querySelector<HTMLButtonElement>(".nav-btn.active[data-nav]");
  const nav = activeBtn?.dataset.nav as ViewId | undefined;
  return nav && VIEW_ORDER.includes(nav) ? nav : "play";
}

function applyPreferences(): void {
  const lang = getEffectiveLanguage();
  applyStaticI18n(lang);

  for (const btn of Array.from(document.querySelectorAll<HTMLButtonElement>("[data-lang-choice]"))) {
    const active = btn.dataset.langChoice === lang;
    btn.classList.toggle("active", active);
    btn.setAttribute("aria-pressed", active ? "true" : "false");
  }
  for (const btn of Array.from(document.querySelectorAll<HTMLButtonElement>("[data-corner-choice]"))) {
    const key = btn.dataset.cornerChoice as CornerStyle | undefined;
    if (key) btn.textContent = CORNER_LABELS[lang][key];
  }
  for (const span of Array.from(document.querySelectorAll<HTMLElement>("[data-accent-label]"))) {
    const key = span.dataset.accentLabel as AccentTone | undefined;
    if (key) span.textContent = ACCENT_LABELS[lang][key];
  }

  const srvNameInput = document.getElementById("srv-name");
  if (srvNameInput instanceof HTMLInputElement && document.activeElement !== srvNameInput) {
    const val = srvNameInput.value.trim();
    if (val === "Yerel Sunucu" || val === "Local Server" || val === "Lokaler Server" || !val) {
      srvNameInput.value = tr(lang, "servers.defaultName");
    }
  }

  byId<HTMLElement>("topbar-crumb").textContent = tr(lang, `view.${currentActiveViewId()}`);

  if (!state) return;
  if (state.config.ui_layout !== "DeckMode") {
    lastDesktopLayout = state.config.ui_layout;
  }

  const theme = getEffectiveTheme();
  const accent = getEffectiveAccent();
  const customHex = getEffectiveCustomHex();
  const corner = getEffectiveCorner();
  const layout = getEffectiveLayout();
  const sounds = getEffectiveSounds();

  setSoundsEnabled(sounds);
  document.documentElement.setAttribute("data-theme", THEME_ATTRS[theme] ?? "dark");
  document.documentElement.setAttribute("data-layout", LAYOUT_ATTRS[layout] ?? "sidebar");
  applyDynamicDesignTokens(theme, accent, customHex, corner);
  document.body.classList.toggle("compact", state.config.ui_compact);
  document.body.classList.toggle("reduce-motion", state.config.ui_reduced_motion);

  byId<HTMLSpanElement>("rail-theme-label").textContent = THEME_SHORT_LABELS[lang][theme];
  byId<HTMLSpanElement>("rail-layout-label").textContent = LAYOUT_SHORT_LABELS[lang][layout];
  byId<HTMLSpanElement>("rail-sound-label").textContent = tr(
    lang,
    sounds ? "rail.soundOn" : "rail.soundOff"
  );
  byId<HTMLButtonElement>("btn-rail-sound").setAttribute("aria-pressed", sounds ? "true" : "false");

  const isDeck = layout === "DeckMode";
  byId<HTMLElement>("deck-controls-bar").hidden = !isDeck;
  byId<HTMLButtonElement>("btn-exit-deck").hidden = !isDeck;
  byId<HTMLButtonElement>("btn-enter-deck").hidden = isDeck;
  const deckMeta = document.querySelector<HTMLElement>(".deck-console-meta");
  if (deckMeta) deckMeta.hidden = !isDeck;
  byId<HTMLElement>("deck-clock").hidden = !isDeck;
  byId<HTMLElement>("console-home").hidden = !isDeck;
  syncDeckClock(isDeck);
  syncDeckGamepadLoop(isDeck);
}

function setActiveView(view: ViewId, updateHash = true, withSound = false): void {
  let changed = false;
  const navButtons = document.querySelectorAll<HTMLButtonElement>(".nav-btn[data-nav]");
  for (const btn of Array.from(navButtons)) {
    const isMatch = btn.dataset.nav === view;
    if (isMatch && !btn.classList.contains("active")) {
      changed = true;
    }
    btn.classList.toggle("active", isMatch);
    if (isMatch) {
      btn.setAttribute("aria-current", "page");
    } else {
      btn.removeAttribute("aria-current");
    }
  }

  const sections = document.querySelectorAll<HTMLElement>("main .view[data-view]");
  for (const section of Array.from(sections)) {
    section.hidden = section.dataset.view !== view;
  }

  byId<HTMLElement>("topbar-crumb").textContent = tr(getEffectiveLanguage(), `view.${view}`);

  if (updateHash && window.location.hash !== `#${view}`) {
    window.history.replaceState(null, "", `#${view}`);
  }
  if (view === "play" && getEffectiveLayout() === "DeckMode") {
    renderConsoleHome();
  }
  if (withSound && changed) {
    playMcSound("tab");
  }
}

function readInitialViewFromHash(): ViewId {
  const raw = window.location.hash.replace(/^#/, "").trim();
  if (raw === "worlds" || raw === "profile" || raw === "system" || raw === "play") {
    return raw;
  }
  return "play";
}

function appendLogLine(line: string, isErr: boolean): void {
  const clean = line.replace(/\r$/, "");
  if (!clean) return;
  const prefix = isErr ? "[ERR] " : "";
  logLines.push(`${prefix}${clean}`);
  if (logLines.length > MAX_LOG_LINES) {
    logLines = logLines.slice(-MAX_LOG_LINES);
  }
  const box = byId<HTMLPreElement>("gradle-log-box");
  box.textContent = logLines.join("\n");
  box.hidden = false;
  box.scrollTop = box.scrollHeight;
}

function renderTopbarAndSession(): void {
  if (!state) return;
  const lang = getEffectiveLanguage();
  byId<HTMLSpanElement>("preview-mode-badge").hidden = ipc.isTauriRuntime();
  byId<HTMLSpanElement>("topbar-username").textContent = state.config.username;
  byId<HTMLSpanElement>("deck-username").textContent = state.config.username;
  drawTopbarHead(byId<HTMLCanvasElement>("topbar-head-canvas"), state.skin_rgba);
  drawTopbarHead(byId<HTMLCanvasElement>("deck-head-canvas"), state.skin_rgba);

  const railStatusRow = byId<HTMLDivElement>("rail-status-row");
  const railStatusText = byId<HTMLSpanElement>("rail-status-text");
  const pill = byId<HTMLDivElement>("engine-status-pill");
  const launchBtn = byId<HTMLButtonElement>("btn-launch-client");
  const prepareBtn = byId<HTMLButtonElement>("btn-prepare-gradle");
  const openMcBtn = byId<HTMLButtonElement>("btn-open-mc-for-world");
  const installJavaBtn = byId<HTMLButtonElement>("btn-install-java");
  const recheckJavaBtn = byId<HTMLButtonElement>("btn-recheck-java");
  const sysInstallJavaBtn = byId<HTMLButtonElement>("btn-system-install-java");
  const sysRecheckJavaBtn = byId<HTMLButtonElement>("btn-system-recheck-java");
  const launchLabel = byId<HTMLSpanElement>("launch-btn-label");

  if (state.running_session) {
    const kind = state.running_session.kind;
    const isPrepare = kind === "prepare";
    const isJavaInstall = kind === "java-install";
    railStatusRow.setAttribute("data-state", "ready");
    railStatusText.textContent = tr(
      lang,
      isJavaInstall ? "rail.statusDownloadingJava" : "rail.statusSessionActive"
    );
    pill.setAttribute("data-state", "running");
    pill.textContent = tr(
      lang,
      isJavaInstall ? "pill.javaInstalling" : isPrepare ? "pill.building" : "pill.running"
    );
    launchBtn.disabled = true;
    prepareBtn.disabled = true;
    openMcBtn.disabled = true;
    installJavaBtn.disabled = true;
    recheckJavaBtn.disabled = true;
    sysInstallJavaBtn.disabled = true;
    sysRecheckJavaBtn.disabled = true;
    launchLabel.textContent = tr(
      lang,
      isJavaInstall ? "btn.installingJava" : isPrepare ? "btn.building" : "btn.mcRunning"
    );
  } else {
    installJavaBtn.disabled = false;
    recheckJavaBtn.disabled = false;
    sysInstallJavaBtn.disabled = false;
    sysRecheckJavaBtn.disabled = false;
    if (!state.vanilla.java_ready) {
      railStatusRow.setAttribute("data-state", "warning");
      railStatusText.textContent = tr(lang, "rail.statusJavaMissing");
      pill.setAttribute("data-state", "warning");
      pill.textContent = tr(lang, "pill.javaRequired");
      launchBtn.disabled = true;
      prepareBtn.disabled = true;
      openMcBtn.disabled = true;
      launchLabel.textContent = tr(lang, "btn.installJavaFirst");
    } else {
      const cacheReady = state.vanilla.client_ready && state.vanilla.assets_ready;
      railStatusRow.setAttribute("data-state", "ready");
      railStatusText.textContent = tr(lang, "rail.statusReady");
      pill.setAttribute("data-state", "idle");
      pill.textContent = tr(lang, cacheReady ? "pill.ready" : "pill.awaitFirstBuild");
      launchBtn.disabled = false;
      prepareBtn.disabled = false;
      openMcBtn.disabled = false;
      launchLabel.textContent = tr(
        lang,
        state.config.last_save_id ? "btn.launchSelected" : "btn.launchMc"
      );
    }
  }
}

function renderPlayView(): void {
  if (!state) return;
  const lang = getEffectiveLanguage();
  const { config, saves, hardware, vanilla } = state;
  const cacheReady = vanilla.client_ready && vanilla.assets_ready;

  const selectedSave = saves.find((s) => s.id === config.last_save_id) ?? null;
  const activeSaveId = selectedSave ? selectedSave.id : null;
  if (config.last_save_id !== activeSaveId) {
    config.last_save_id = activeSaveId;
  }

  byId<HTMLHeadingElement>("play-world-title").textContent =
    activeSaveId ?? tr(lang, "play.mainMenuTitle");
  byId<HTMLDivElement>("stage-biome-tag").textContent = tr(
    lang,
    activeSaveId ? "play.stageTagSp" : "play.stageTagDefault"
  );

  const stratumSvg = document.getElementById("stratum-svg");
  if (stratumSvg instanceof SVGSVGElement) {
    renderClientStratumSvg(stratumSvg, activeSaveId ?? "basalt-1-21-4", true);
  }

  const select = byId<HTMLSelectElement>("play-save-select");
  select.innerHTML = "";
  const titleOpt = document.createElement("option");
  titleOpt.value = "";
  titleOpt.textContent = tr(lang, "play.mainMenuOpt");
  select.appendChild(titleOpt);

  for (const save of saves) {
    const opt = document.createElement("option");
    opt.value = save.id;
    opt.textContent = `${save.id} (${formatBytes(save.size_bytes)})`;
    select.appendChild(opt);
  }
  select.value = activeSaveId ?? "";

  byId<HTMLElement>("play-spec-jdk").textContent = vanilla.java_ready
    ? "Java 21"
    : tr(lang, "play.specJdkMissing");
  byId<HTMLElement>("play-spec-ram").textContent = `${config.graphics.jvm_ram_mb} MB (${config.graphics.jvm_gc})`;
  byId<HTMLElement>("play-spec-chunks").textContent = `${config.graphics.render_distance} chunk`;
  byId<HTMLElement>("play-spec-mod").textContent = tr(
    lang,
    cacheReady ? "play.specModReady" : "play.specModAuto"
  );

  const javaPanel = byId<HTMLDivElement>("java-setup-panel");
  if (!vanilla.java_ready) {
    javaPanel.hidden = false;
    byId<HTMLParagraphElement>("java-setup-desc").textContent =
      lang === "En" ? vanilla.java_hint : tr(lang, "play.javaSetupDesc");
  } else {
    javaPanel.hidden = true;
  }

  const gpuShort = hardware.gpu_name.length > 34 ? `${hardware.gpu_name.slice(0, 31)}...` : hardware.gpu_name;
  byId<HTMLSpanElement>("play-hw-summary").textContent =
    `${hardware.physical_cores}C/${hardware.logical_cores}T · ${Math.round(hardware.total_ram_mb / 1024)} GB RAM · ${gpuShort} (${config.graphics.preset})`;

  const playUsername = byId<HTMLInputElement>("play-username");
  if (document.activeElement !== playUsername) {
    playUsername.value = config.username;
  }

  byId<HTMLSpanElement>("prepare-status-hint").textContent = tr(
    lang,
    cacheReady ? "play.prepareHintReady" : "play.prepareHintWait"
  );

  if (getEffectiveLayout() === "DeckMode") {
    renderConsoleHome();
  }
}

function buildDeckCards(): DeckCard[] {
  if (!state) return [];
  const lang = getEffectiveLanguage();
  const { config, saves, hardware, vanilla } = state;
  const cacheReady = vanilla.client_ready && vanilla.assets_ready;
  const cards: DeckCard[] = [
    {
      id: "main-menu",
      kind: "launch",
      kicker: tr(lang, "deck.cardMainKicker"),
      title: tr(lang, "play.mainMenuTitle"),
      meta: tr(lang, cacheReady ? "deck.cardMainMetaReady" : "deck.cardMainMetaAuto"),
      sub: tr(lang, "deck.cardMainSub", {
        user: config.username,
        preset: config.graphics.preset,
      }),
      tag: tr(lang, "deck.cardMainTag"),
      seedKey: "basalt-console-home",
      primaryLabel: tr(lang, "deck.cardMainPrimary"),
      secondaryLabel: tr(lang, "play.manageSaves"),
      launchMode: "title",
      saveId: null,
      targetView: "worlds",
    },
  ];

  for (const save of saves) {
    const isActive = save.id === config.last_save_id;
    cards.push({
      id: `world:${save.id}`,
      kind: "world",
      kicker: tr(
        lang,
        isActive ? "deck.cardWorldKickerActive" : "deck.cardWorldKicker"
      ),
      title: save.id,
      meta: formatBytes(save.size_bytes),
      sub: tr(lang, "deck.cardWorldSub", {
        date: formatEpoch(save.modified_epoch_secs),
      }),
      tag: `QUICK PLAY · ${save.id}`,
      seedKey: save.id,
      primaryLabel: tr(lang, "deck.cardWorldPrimary"),
      secondaryLabel: tr(
        lang,
        isActive ? "deck.cardWorldSecActive" : "deck.cardWorldSecPick"
      ),
      secondaryDisabled: isActive,
      launchMode: "singleplayer",
      saveId: save.id,
    });
  }

  for (const srv of config.saved_servers) {
    const check = serverChecks.get(srv.address);
    const metaText =
      check?.status === "online" && check.ping
        ? `${check.ping.latency_ms} ms · ${check.ping.online_players}/${check.ping.max_players}`
        : srv.address;
    cards.push({
      id: `server:${srv.address}`,
      kind: "server",
      kicker: tr(lang, "deck.cardServerKicker"),
      title: srv.name,
      meta: metaText,
      sub: tr(lang, "deck.cardServerSub", { addr: srv.address }),
      tag: tr(lang, "deck.cardServerTag", { addr: srv.address }),
      seedKey: `srv-${srv.address}`,
      primaryLabel: tr(lang, "deck.cardServerPrimary"),
      secondaryLabel: tr(lang, "nav.system"),
      launchMode: "multiplayer",
      serverAddress: srv.address,
      targetView: "system",
    });
  }

  cards.push(
    {
      id: "nav:worlds",
      kind: "nav",
      kicker: tr(lang, "worlds.heading").toUpperCase(),
      title: tr(lang, "worlds.heading"),
      meta: tr(lang, "worlds.countLabel", { count: saves.length }),
      sub: tr(lang, "deck.cardNavWorldsSub"),
      tag: tr(lang, "deck.cardNavWorldsTag"),
      seedKey: "nav-worlds",
      primaryLabel: tr(lang, "deck.cardNavWorldsPrimary"),
      secondaryLabel: tr(lang, "nav.profile"),
      targetView: "worlds",
    },
    {
      id: "nav:profile",
      kind: "nav",
      kicker: tr(lang, "deck.cardNavProfileKicker"),
      title: tr(lang, "deck.cardNavProfileTitle"),
      meta: tr(lang, "deck.cardNavProfileMeta", {
        theme: THEME_SHORT_LABELS[lang][getEffectiveTheme()],
      }),
      sub: tr(lang, "deck.cardNavProfileSub"),
      tag: tr(lang, "nav.profile").toUpperCase(),
      seedKey: "nav-profile",
      primaryLabel: tr(lang, "deck.cardNavProfilePrimary"),
      secondaryLabel: tr(lang, "play.manageSaves"),
      targetView: "profile",
    },
    {
      id: "nav:system",
      kind: "nav",
      kicker: tr(lang, "system.heading").toUpperCase(),
      title: tr(lang, "deck.cardNavSystemTitle"),
      meta: `${hardware.physical_cores}C/${hardware.logical_cores}T · ${Math.round(hardware.total_ram_mb / 1024)} GB`,
      sub: tr(lang, "deck.cardNavSystemSub"),
      tag: tr(lang, "nav.system").toUpperCase(),
      seedKey: "nav-system",
      primaryLabel: tr(lang, "deck.cardNavSystemPrimary"),
      secondaryLabel: tr(lang, "deck.cardNavProfilePrimary"),
      targetView: "system",
    }
  );

  return cards;
}

function appendHeroSpec(dl: HTMLElement, term: string, val: string): void {
  const row = document.createElement("div");
  const dt = document.createElement("dt");
  dt.textContent = term;
  const dd = document.createElement("dd");
  dd.className = "tabular";
  dd.textContent = val;
  row.append(dt, dd);
  dl.appendChild(row);
}

function renderConsoleHero(card: DeckCard): void {
  if (!state) return;
  const lang = getEffectiveLanguage();
  const { config, vanilla, hardware, running_session, skin_rgba, skin_is_slim } = state;

  const heroSvg = document.getElementById("deck-hero-svg");
  if (heroSvg instanceof SVGSVGElement) {
    renderClientStratumSvg(heroSvg, card.seedKey, true);
  }

  byId<HTMLElement>("deck-hero-kicker").textContent = card.kicker;
  byId<HTMLElement>("deck-hero-tag").textContent = running_session
    ? `PID ${running_session.pid}`
    : card.tag;
  byId<HTMLElement>("deck-hero-title").textContent = card.title;
  byId<HTMLElement>("deck-hero-sub").textContent = card.sub;

  const specsEl = byId<HTMLElement>("deck-hero-specs");
  specsEl.innerHTML = "";
  appendHeroSpec(
    specsEl,
    "JDK",
    vanilla.java_ready ? "Java 21" : tr(lang, "play.specJdkMissing")
  );
  appendHeroSpec(specsEl, "JVM RAM", `${config.graphics.jvm_ram_mb} MB`);
  appendHeroSpec(
    specsEl,
    tr(lang, "system.renderDist").toUpperCase(),
    `${config.graphics.render_distance} chunk`
  );
  appendHeroSpec(
    specsEl,
    tr(lang, card.kind === "world" ? "deck.sizeLabel" : "deck.hwLabel"),
    card.kind === "world"
      ? card.meta
      : `${hardware.physical_cores}C/${hardware.logical_cores}T`
  );

  const primaryBtn = byId<HTMLButtonElement>("btn-deck-primary");
  const primaryLabel = byId<HTMLSpanElement>("deck-primary-label");
  const needsJava = !vanilla.java_ready && Boolean(card.launchMode);
  primaryLabel.textContent = needsJava ? tr(lang, "deck.installJavaBtn") : card.primaryLabel;
  primaryBtn.disabled = running_session !== null && Boolean(card.launchMode);

  const secondaryBtn = byId<HTMLButtonElement>("btn-deck-secondary");
  byId<HTMLSpanElement>("deck-secondary-label").textContent = card.secondaryLabel;
  secondaryBtn.disabled = Boolean(card.secondaryDisabled);

  byId<HTMLButtonElement>("btn-deck-prepare").disabled =
    running_session !== null || !vanilla.java_ready;

  const statusLine = byId<HTMLElement>("deck-status-line");
  if (running_session) {
    statusLine.textContent = tr(lang, "deck.statusRunning", {
      label: running_session.label,
      pid: running_session.pid,
    });
  } else if (!vanilla.java_ready) {
    statusLine.textContent = tr(lang, "deck.statusMissingJava");
  } else if (config.last_save_id) {
    statusLine.textContent = tr(lang, "deck.statusReadyWorld", {
      save: config.last_save_id,
      preset: config.graphics.preset,
    });
  } else {
    statusLine.textContent = tr(lang, "deck.statusReadyProfile", {
      user: config.username,
      preset: config.graphics.preset,
      ram: config.graphics.jvm_ram_mb,
    });
  }

  drawSkinPreview(byId<HTMLCanvasElement>("deck-skin-canvas"), skin_rgba, "front", skin_is_slim);
  byId<HTMLElement>("deck-skin-caption").textContent = config.username;
  byId<HTMLElement>("deck-skin-model-badge").textContent = tr(
    lang,
    skin_is_slim ? "skin.slimShort" : "skin.classicShort"
  );
}

function selectDeckCard(
  index: number,
  opts: { scroll?: boolean; focusBtn?: boolean; withSound?: boolean } = {}
): void {
  if (deckCards.length === 0) return;
  const clamped = ((index % deckCards.length) + deckCards.length) % deckCards.length;
  const changed = clamped !== selectedDeckIdx;
  selectedDeckIdx = clamped;
  const card = deckCards[selectedDeckIdx];
  renderConsoleHero(card);

  const carousel = byId<HTMLDivElement>("deck-carousel");
  const buttons = Array.from(carousel.querySelectorAll<HTMLButtonElement>(".deck-card"));
  buttons.forEach((btn, idx) => {
    const selected = idx === selectedDeckIdx;
    btn.classList.toggle("is-selected", selected);
    btn.setAttribute("aria-selected", selected ? "true" : "false");
  });

  const activeCardBtn = buttons[selectedDeckIdx];
  if (activeCardBtn) {
    if (opts.scroll) {
      activeCardBtn.scrollIntoView({
        behavior: state?.config.ui_reduced_motion ? "auto" : "smooth",
        block: "nearest",
        inline: "center",
      });
    }
    if (opts.focusBtn) {
      activeCardBtn.focus({ preventScroll: true });
    }
  }
  if (opts.withSound && changed) {
    playMcSound("focus");
  }
}

function handleDeckPrimaryAction(card: DeckCard): void {
  if (!state) return;
  unlockAudioContext();
  if (card.launchMode) {
    if (!state.vanilla.java_ready) {
      playMcSound("click");
      void handleInstallJava21();
      return;
    }
    void handleLaunchClient(card.launchMode, card.saveId ?? null, card.serverAddress ?? null);
    return;
  }
  if (card.targetView) {
    setActiveView(card.targetView, true, true);
  }
}

function handleDeckSecondaryAction(card: DeckCard): void {
  unlockAudioContext();
  if (card.kind === "world" && card.saveId) {
    playMcSound("click");
    void handleSelectSave(card.saveId);
    return;
  }
  if (card.targetView) {
    setActiveView(card.targetView, true, true);
    return;
  }
  setActiveView("worlds", true, true);
}

function renderConsoleHome(): void {
  if (!state) return;
  const lang = getEffectiveLanguage();
  const prevId = deckCards[selectedDeckIdx]?.id ?? null;
  deckCards = buildDeckCards();
  if (prevId) {
    const foundIdx = deckCards.findIndex((c) => c.id === prevId);
    selectedDeckIdx = foundIdx >= 0 ? foundIdx : 0;
  } else {
    selectedDeckIdx = 0;
  }

  byId<HTMLElement>("deck-carousel-count").textContent = tr(lang, "deck.itemsCount", {
    count: deckCards.length,
  });
  const carousel = byId<HTMLDivElement>("deck-carousel");
  carousel.innerHTML = "";

  deckCards.forEach((card, idx) => {
    const btn = document.createElement("button");
    btn.type = "button";
    btn.className = "deck-card";
    btn.dataset.kind = card.kind;
    btn.dataset.cardIndex = String(idx);
    btn.setAttribute("role", "option");
    btn.setAttribute("aria-selected", idx === selectedDeckIdx ? "true" : "false");
    if (idx === selectedDeckIdx) {
      btn.classList.add("is-selected");
    }

    const coverSvg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
    coverSvg.setAttribute("class", "deck-card-cover");
    coverSvg.setAttribute("viewBox", "0 0 240 84");
    coverSvg.setAttribute("preserveAspectRatio", "xMidYMid slice");
    coverSvg.setAttribute("aria-hidden", "true");
    renderDeckCardThumbSvg(coverSvg, card.seedKey, card.kind);

    const body = document.createElement("span");
    body.className = "deck-card-body";

    const kicker = document.createElement("span");
    kicker.className = "deck-card-kicker";
    kicker.textContent = card.kicker;

    const title = document.createElement("span");
    title.className = "deck-card-title";
    title.textContent = card.title;

    const meta = document.createElement("span");
    meta.className = "deck-card-meta";
    meta.textContent = card.meta;

    body.append(kicker, title, meta);
    btn.append(coverSvg, body);

    btn.addEventListener("click", () => {
      unlockAudioContext();
      if (selectedDeckIdx !== idx) {
        selectDeckCard(idx, { scroll: true, focusBtn: false, withSound: true });
      } else {
        handleDeckPrimaryAction(card);
      }
    });
    btn.addEventListener("dblclick", () => {
      unlockAudioContext();
      selectDeckCard(idx, { scroll: true, focusBtn: false, withSound: false });
      handleDeckPrimaryAction(card);
    });
    btn.addEventListener("focus", () => {
      if (selectedDeckIdx !== idx) {
        selectDeckCard(idx, { scroll: true, focusBtn: false, withSound: false });
      }
    });

    carousel.appendChild(btn);
  });

  if (deckCards.length > 0) {
    renderConsoleHero(deckCards[selectedDeckIdx]);
  }
}

function enterConsoleMode(): void {
  if (!state) return;
  unlockAudioContext();
  if (state.config.ui_layout !== "DeckMode") {
    lastDesktopLayout = state.config.ui_layout;
  }
  consoleModeActive = true;
  consoleAudioMuted = false;
  setSoundsEnabled(true);
  playMcSound("launch", true);
  applyPreferences();
  renderProfileView();
  setActiveView("play", true, false);
  renderConsoleHome();
  void ipc.setFullscreen(true).catch(() => undefined);
  window.requestAnimationFrame(() => {
    const heroBtn = document.getElementById("btn-deck-primary");
    if (heroBtn instanceof HTMLButtonElement && !heroBtn.disabled) {
      heroBtn.focus({ preventScroll: true });
    }
  });
}

function toggleUiSound(desired?: boolean): void {
  if (!state) return;
  unlockAudioContext();
  if (getEffectiveLayout() === "DeckMode") {
    const nextMuted = desired !== undefined ? !desired : !consoleAudioMuted;
    consoleAudioMuted = nextMuted;
    const nowEnabled = !consoleAudioMuted;
    setSoundsEnabled(nowEnabled);
    playMcSound(nowEnabled ? "toggleOn" : "toggleOff", true);
    applyPreferences();
    renderProfileView();
    return;
  }
  const nextDesktopSound = desired !== undefined ? desired : !state.config.ui_sounds;
  void saveAccessibilityPrefs({ ui_sounds: nextDesktopSound });
}

function renderWorldsView(): void {
  if (!state) return;
  const lang = getEffectiveLanguage();
  const { saves, config, vanilla } = state;

  byId<HTMLSpanElement>("worlds-count-label").textContent = tr(lang, "worlds.countLabel", {
    count: saves.length,
  });
  const banner = byId<HTMLDivElement>("worlds-state-banner");
  const list = byId<HTMLDivElement>("worlds-list");
  list.innerHTML = "";

  if (saves.length === 0) {
    setBanner(
      banner,
      tr(lang, "worlds.emptyBanner", { dir: vanilla.saves_dir }),
      "neutral"
    );
    return;
  }

  setBanner(banner, null);

  for (const save of saves) {
    const isActive = config.last_save_id === save.id;
    const row = document.createElement("div");
    row.className = isActive ? "world-row is-active" : "world-row";
    row.setAttribute("role", "listitem");

    const main = document.createElement("div");
    main.className = "row-main";

    const title = document.createElement("div");
    title.className = "row-title";
    const nameSpan = document.createElement("span");
    nameSpan.textContent = save.id;
    title.appendChild(nameSpan);

    if (isActive) {
      const tag = document.createElement("span");
      tag.className = "row-tag";
      tag.textContent = tr(lang, "worlds.selectedTag");
      title.appendChild(tag);
    }

    const sub = document.createElement("div");
    sub.className = "row-sub tabular";
    sub.textContent = tr(lang, "worlds.metaLine", {
      size: formatBytes(save.size_bytes),
      date: formatEpoch(save.modified_epoch_secs),
    });

    main.append(title, sub);

    const actions = document.createElement("div");
    actions.className = "row-actions";

    const selectBtn = document.createElement("button");
    selectBtn.type = "button";
    selectBtn.className = "btn-secondary btn-sm";
    selectBtn.textContent = tr(lang, isActive ? "worlds.btnSelected" : "worlds.btnSelect");
    selectBtn.disabled = isActive;
    selectBtn.addEventListener("click", () => {
      void handleSelectSave(save.id);
    });

    const playBtn = document.createElement("button");
    playBtn.type = "button";
    playBtn.className = "btn-primary btn-sm";
    playBtn.textContent = tr(lang, "worlds.btnLaunch");
    playBtn.disabled = state?.running_session !== null;
    playBtn.addEventListener("click", () => {
      void handleLaunchClient("singleplayer", save.id, null);
    });

    const deleteBtn = document.createElement("button");
    deleteBtn.type = "button";
    deleteBtn.className = "btn-danger btn-sm";
    deleteBtn.textContent = tr(lang, "worlds.btnDelete");
    deleteBtn.disabled = state?.running_session !== null;
    deleteBtn.addEventListener("click", () => {
      openDeleteDialog(save, deleteBtn);
    });

    actions.append(selectBtn, playBtn, deleteBtn);
    row.append(main, actions);
    list.appendChild(row);
  }
}

function renderProfileView(): void {
  if (!state) return;
  const lang = getEffectiveLanguage();
  const { config, skin_rgba, skin_is_slim } = state;

  const usernameInput = byId<HTMLInputElement>("profile-username");
  if (document.activeElement !== usernameInput) {
    usernameInput.value = config.username;
  }

  const effectiveTheme = getEffectiveTheme();
  const effectiveAccent = getEffectiveAccent();
  const effectiveCustomHex = getEffectiveCustomHex();
  const effectiveCorner = getEffectiveCorner();
  const effectiveLayout = getEffectiveLayout();

  for (const btn of Array.from(document.querySelectorAll<HTMLButtonElement>("[data-theme-choice]"))) {
    btn.setAttribute("aria-pressed", btn.dataset.themeChoice === effectiveTheme ? "true" : "false");
  }
  for (const btn of Array.from(document.querySelectorAll<HTMLButtonElement>("[data-layout-choice]"))) {
    btn.setAttribute("aria-pressed", btn.dataset.layoutChoice === effectiveLayout ? "true" : "false");
  }
  for (const btn of Array.from(document.querySelectorAll<HTMLButtonElement>("[data-accent-choice]"))) {
    btn.setAttribute("aria-pressed", btn.dataset.accentChoice === effectiveAccent ? "true" : "false");
  }
  for (const btn of Array.from(document.querySelectorAll<HTMLButtonElement>("[data-corner-choice]"))) {
    const active = btn.dataset.cornerChoice === effectiveCorner;
    btn.classList.toggle("active", active);
    btn.setAttribute("aria-pressed", active ? "true" : "false");
  }
  byId<HTMLLabelElement>("pref-custom-chip").classList.toggle("is-active", effectiveAccent === "Custom");
  const colorInput = byId<HTMLInputElement>("pref-custom-accent");
  if (document.activeElement !== colorInput) {
    colorInput.value = effectiveCustomHex;
  }
  byId<HTMLSpanElement>("pref-custom-hex-label").textContent = effectiveCustomHex;
  byId<HTMLInputElement>("pref-compact").checked = config.ui_compact;
  byId<HTMLInputElement>("pref-reduced-motion").checked = config.ui_reduced_motion;
  byId<HTMLInputElement>("pref-sounds").checked = getEffectiveSounds();

  byId<HTMLSpanElement>("skin-resolved-badge").textContent = tr(
    lang,
    skin_is_slim ? "skin.badgeSlim" : "skin.badgeClassic"
  );
  byId<HTMLSelectElement>("skin-model-select").value = config.skin_model;
  byId<HTMLParagraphElement>("skin-file-label").textContent =
    config.skin_file ?? tr(lang, "skin.defaultFileLabel");

  drawSkinPreview(byId<HTMLCanvasElement>("skin-preview-canvas"), skin_rgba, skinViewMode, skin_is_slim);

  const viewBtns = document.querySelectorAll<HTMLButtonElement>("[data-skin-view]");
  for (const btn of Array.from(viewBtns)) {
    btn.classList.toggle("active", btn.dataset.skinView === skinViewMode);
  }
}

function renderVanillaStatus(vanilla: VanillaEnvStatus): void {
  const lang = getEffectiveLanguage();
  const ready = vanilla.java_ready && vanilla.client_ready && vanilla.assets_ready;
  byId<HTMLSpanElement>("gradle-status-tag").textContent = tr(
    lang,
    !vanilla.java_ready
      ? "system.tagMissingJava"
      : ready
      ? "system.tagReady"
      : "system.tagJavaReady"
  );
  byId<HTMLElement>("gradle-java-label").textContent = vanilla.java_ready
    ? vanilla.java_version_label
    : lang === "En"
    ? vanilla.java_version_label
    : tr(lang, "play.specJdkMissing");
  byId<HTMLElement>("gradle-jar-status").textContent = tr(
    lang,
    vanilla.client_ready && vanilla.assets_ready
      ? "system.modJarBuilt"
      : "system.modJarAuto"
  );
  byId<HTMLElement>("gradle-dir-label").textContent = vanilla.vanilla_dir;
  byId<HTMLElement>("gradle-saves-dir").textContent = vanilla.saves_dir;
  byId<HTMLButtonElement>("btn-system-install-java").hidden = vanilla.java_ready;
  const hwDrawer = document.getElementById("hardware-runtime-drawer");
  if (hwDrawer instanceof HTMLDetailsElement && !vanilla.java_ready) {
    hwDrawer.open = true;
  }
}

function renderServersList(): void {
  if (!state) return;
  const lang = getEffectiveLanguage();
  const list = byId<HTMLDivElement>("servers-list");
  list.innerHTML = "";
  const servers = state.config.saved_servers;

  if (servers.length === 0) {
    const empty = document.createElement("p");
    empty.className = "muted-copy";
    empty.textContent = tr(lang, "servers.empty");
    list.appendChild(empty);
    return;
  }

  servers.forEach((srv: SavedServer, index: number) => {
    const check = serverChecks.get(srv.address) ?? { status: "idle" as const };

    const row = document.createElement("div");
    row.className = "server-row";

    const main = document.createElement("div");
    main.className = "row-main";

    const title = document.createElement("div");
    title.className = "row-title";
    const nameEl = document.createElement("span");
    nameEl.textContent = srv.name;
    title.appendChild(nameEl);

    if (check.status === "online" && check.ping) {
      const tag = document.createElement("span");
      tag.className = "row-tag tabular";
      tag.textContent = `${check.ping.latency_ms} ms · ${check.ping.online_players}/${check.ping.max_players}`;
      title.appendChild(tag);
    } else if (check.status === "error") {
      const tag = document.createElement("span");
      tag.className = "row-tag";
      tag.textContent = tr(lang, "servers.noResponse");
      title.appendChild(tag);
    }

    const sub = document.createElement("div");
    sub.className = "row-sub tabular";
    if (check.status === "online" && check.ping) {
      sub.textContent = tr(lang, "servers.subOnline", {
        addr: srv.address,
        ver: check.ping.version_name,
        proto: check.ping.protocol_version,
        desc: check.ping.description,
      });
    } else if (check.status === "error" && check.message) {
      sub.textContent = `${srv.address} · ${check.message}`;
    } else if (check.status === "checking") {
      sub.textContent = tr(lang, "servers.subChecking", { addr: srv.address });
    } else {
      sub.textContent = srv.address;
    }

    main.append(title, sub);

    const actions = document.createElement("div");
    actions.className = "row-actions";

    const pingBtn = document.createElement("button");
    pingBtn.type = "button";
    pingBtn.className = "btn-secondary btn-sm";
    pingBtn.textContent = tr(
      lang,
      check.status === "checking" ? "servers.btnChecking" : "servers.btnPing"
    );
    pingBtn.disabled = check.status === "checking";
    pingBtn.addEventListener("click", () => {
      void handlePingServer(srv.address);
    });

    const probeBtn = document.createElement("button");
    probeBtn.type = "button";
    probeBtn.className = "btn-secondary btn-sm";
    probeBtn.textContent = tr(lang, "servers.btnProbe");
    probeBtn.addEventListener("click", () => {
      void handleProbeServer(srv.address);
    });

    const connectBtn = document.createElement("button");
    connectBtn.type = "button";
    connectBtn.className = "btn-primary btn-sm";
    connectBtn.textContent = tr(lang, "servers.btnConnect");
    connectBtn.disabled = state?.running_session !== null;
    connectBtn.addEventListener("click", () => {
      void handleLaunchClient("multiplayer", null, srv.address);
    });

    const removeBtn = document.createElement("button");
    removeBtn.type = "button";
    removeBtn.className = "btn-danger btn-sm";
    removeBtn.textContent = tr(lang, "servers.btnRemove");
    removeBtn.addEventListener("click", () => {
      void handleRemoveServer(index);
    });

    actions.append(pingBtn, probeBtn, connectBtn, removeBtn);
    row.append(main, actions);
    list.appendChild(row);
  });
}

function renderSystemView(): void {
  if (!state) return;
  const lang = getEffectiveLanguage();
  const { hardware, config, vanilla, hardware_notice } = state;

  setBanner(byId<HTMLDivElement>("hardware-optimize-notice"), hardware_notice, "neutral");

  const tierLabel = GPU_TIER_LABELS[lang][hardware.gpu_device_type];
  byId<HTMLSpanElement>("hw-tier-tag").textContent = tierLabel;
  byId<HTMLElement>("hw-cpu").textContent = hardware.cpu_name;
  byId<HTMLElement>("hw-cores").textContent = tr(lang, "system.coresVal", {
    phys: hardware.physical_cores,
    log: hardware.logical_cores,
  });
  byId<HTMLElement>("hw-ram").textContent = `${hardware.available_ram_mb} MB / ${hardware.total_ram_mb} MB`;
  byId<HTMLElement>("hw-gpu").textContent = `${hardware.gpu_name} (${hardware.gpu_backend})`;

  const gfx = config.graphics;
  byId<HTMLSpanElement>("gfx-preset-badge").textContent = tr(
    lang,
    gfx.auto_configured ? "system.presetAuto" : "system.presetManual",
    { preset: gfx.preset }
  );

  byId<HTMLInputElement>("gfx-render-dist").value = String(gfx.render_distance);
  byId<HTMLOutputElement>("gfx-render-dist-out").textContent = String(gfx.render_distance);

  byId<HTMLInputElement>("gfx-sim-dist").value = String(gfx.simulation_distance);
  byId<HTMLOutputElement>("gfx-sim-dist-out").textContent = String(gfx.simulation_distance);

  byId<HTMLInputElement>("gfx-fov").value = String(gfx.fov_degrees);
  byId<HTMLOutputElement>("gfx-fov-out").textContent = `${gfx.fov_degrees}°`;

  setSelectNumericValue(byId<HTMLSelectElement>("gfx-jvm-ram"), gfx.jvm_ram_mb);
  byId<HTMLSelectElement>("gfx-jvm-gc").value = gfx.jvm_gc;
  byId<HTMLSelectElement>("gfx-max-fps").value = String(gfx.max_fps);
  byId<HTMLInputElement>("gfx-vsync").checked = gfx.vsync;
  byId<HTMLInputElement>("gfx-fancy").checked = gfx.fancy_graphics;
  byId<HTMLInputElement>("gfx-ao").checked = gfx.smooth_lighting;

  renderVanillaStatus(vanilla);
  renderServersList();
}

function renderAll(): void {
  if (!state) return;
  applyPreferences();
  renderTopbarAndSession();
  renderPlayView();
  renderWorldsView();
  renderProfileView();
  renderSystemView();
}

async function handleSelectSave(saveId: string | null): Promise<void> {
  if (!state) return;
  try {
    await ipc.selectSave(saveId);
    state.config.last_save_id = saveId;
    renderPlayView();
    renderWorldsView();
    renderTopbarAndSession();
  } catch (err) {
    showToast(errorMessage(err), "error");
  }
}

async function commitUsernameFromPlay(): Promise<boolean> {
  if (!state) return false;
  const input = byId<HTMLInputElement>("play-username");
  const errEl = byId<HTMLSpanElement>("play-username-error");
  const validation = validateUsername(input.value);
  if (validation) {
    errEl.textContent = validation;
    errEl.hidden = false;
    input.focus();
    return false;
  }
  errEl.hidden = true;
  const cleaned = input.value.trim();
  if (cleaned === state.config.username) {
    return true;
  }
  state.config = await ipc.saveProfileSettings({
    username: cleaned,
    theme: state.config.theme,
    ui_accent: state.config.ui_accent,
    custom_accent_hex: state.config.custom_accent_hex,
    ui_corner: state.config.ui_corner,
    ui_layout: state.config.ui_layout,
    ui_compact: state.config.ui_compact,
    ui_reduced_motion: state.config.ui_reduced_motion,
    ui_sounds: state.config.ui_sounds,
    ui_language: state.config.ui_language,
  });
  renderTopbarAndSession();
  renderProfileView();
  return true;
}

async function handlePrepareVanilla(): Promise<void> {
  if (!state) return;
  logLines = [];
  const feedback = byId<HTMLDivElement>("launch-feedback");
  setBanner(feedback, null);
  try {
    const session = await ipc.prepareVanillaClient();
    state.running_session = session;
    renderTopbarAndSession();
    renderPlayView();
    renderWorldsView();
    renderServersList();
    playMcSound("launch");
    setBanner(feedback, tr(getEffectiveLanguage(), "play.prebuildStarted"), "neutral");
  } catch (err) {
    playMcSound("error");
    setBanner(feedback, errorMessage(err), "error");
    showToast(errorMessage(err), "error");
  }
}

async function handleInstallJava21(): Promise<void> {
  if (!state) return;
  logLines = [];
  const feedback = byId<HTMLDivElement>("launch-feedback");
  setActiveView("play");
  setBanner(feedback, null);
  try {
    const session = await ipc.installJava21();
    state.running_session = session;
    renderTopbarAndSession();
    renderPlayView();
    renderWorldsView();
    renderServersList();
    setBanner(feedback, tr(getEffectiveLanguage(), "play.javaInstallStarted"), "neutral");
  } catch (err) {
    setBanner(feedback, errorMessage(err), "error");
    showToast(errorMessage(err), "error");
  }
}

async function handleRecheckJava21(): Promise<void> {
  if (!state) return;
  try {
    const status = await ipc.recheckJava21();
    state.vanilla = status;
    renderTopbarAndSession();
    renderPlayView();
    renderSystemView();
    if (status.java_ready) {
      showToast(
        tr(getEffectiveLanguage(), "play.javaDetected", { ver: status.java_version_label }),
        "success"
      );
    } else {
      showToast(status.java_hint, "error");
    }
  } catch (err) {
    showToast(errorMessage(err), "error");
  }
}

async function handleOpenJavaPage(): Promise<void> {
  try {
    await ipc.openJavaDownloadPage();
  } catch (err) {
    showToast(errorMessage(err), "error");
  }
}

async function handleLaunchClient(
  mode: LaunchMode,
  saveId: string | null,
  serverAddress: string | null
): Promise<void> {
  if (!state) return;
  const feedback = byId<HTMLDivElement>("launch-feedback");
  setBanner(feedback, null);

  try {
    if (!(await commitUsernameFromPlay())) {
      return;
    }

    logLines = [];
    const session = await ipc.launchClient({
      mode,
      save_id: saveId,
      server_address: serverAddress,
    });
    state.running_session = session;
    setActiveView("play");
    renderTopbarAndSession();
    renderPlayView();
    renderWorldsView();
    renderServersList();
    playMcSound("launch");
    setBanner(
      feedback,
      tr(getEffectiveLanguage(), "play.launchingNote", {
        label: session.label,
        pid: session.pid,
      }),
      "neutral"
    );
  } catch (err) {
    setActiveView("play");
    playMcSound("error");
    const msg = errorMessage(err);
    setBanner(feedback, msg, "error");
    showToast(msg, "error");
  }
}

async function handleRefreshSaves(): Promise<void> {
  if (!state) return;
  const btn = byId<HTMLButtonElement>("btn-refresh-worlds");
  btn.disabled = true;
  try {
    state.saves = await ipc.listSaves();
    renderPlayView();
    renderWorldsView();
    showToast(tr(getEffectiveLanguage(), "worlds.refreshed"), "success");
  } catch (err) {
    setBanner(byId<HTMLDivElement>("worlds-state-banner"), errorMessage(err), "error");
  } finally {
    btn.disabled = false;
  }
}

function openDeleteDialog(save: MinecraftSaveSummary, trigger: HTMLElement): void {
  pendingDeleteSaveId = save.id;
  lastDialogTrigger = trigger;
  const savesRoot = state?.vanilla.saves_dir ?? "run/saves";
  byId<HTMLParagraphElement>("delete-dialog-desc").textContent = tr(
    getEffectiveLanguage(),
    "worlds.deleteDesc",
    {
      id: save.id,
      size: formatBytes(save.size_bytes),
      root: savesRoot,
    }
  );
  byId<HTMLDialogElement>("delete-world-dialog").showModal();
}

async function confirmDeleteSave(): Promise<void> {
  if (!state || !pendingDeleteSaveId) return;
  const saveId = pendingDeleteSaveId;
  const confirmBtn = byId<HTMLButtonElement>("delete-dialog-confirm");
  confirmBtn.disabled = true;
  try {
    state.saves = await ipc.deleteSave(saveId);
    if (state.config.last_save_id === saveId) {
      const fallback = state.saves[0]?.id ?? null;
      await ipc.selectSave(fallback);
      state.config.last_save_id = fallback;
    }
    byId<HTMLDialogElement>("delete-world-dialog").close();
    renderPlayView();
    renderWorldsView();
    renderTopbarAndSession();
    showToast(tr(getEffectiveLanguage(), "worlds.deletedToast", { id: saveId }), "success");
  } catch (err) {
    showToast(errorMessage(err), "error");
  } finally {
    confirmBtn.disabled = false;
    pendingDeleteSaveId = null;
  }
}

async function handleSaveProfileForm(e: SubmitEvent): Promise<void> {
  e.preventDefault();
  if (!state) return;
  const input = byId<HTMLInputElement>("profile-username");
  const errEl = byId<HTMLSpanElement>("profile-username-error");
  const statusEl = byId<HTMLSpanElement>("profile-save-status");

  const validation = validateUsername(input.value);
  if (validation) {
    errEl.textContent = validation;
    errEl.hidden = false;
    statusEl.textContent = "";
    input.focus();
    return;
  }
  errEl.hidden = true;

  try {
    state.config = await ipc.saveProfileSettings({
      username: input.value.trim(),
      theme: state.config.theme,
      ui_accent: state.config.ui_accent,
      custom_accent_hex: state.config.custom_accent_hex,
      ui_corner: state.config.ui_corner,
      ui_layout: state.config.ui_layout,
      ui_compact: state.config.ui_compact,
      ui_reduced_motion: state.config.ui_reduced_motion,
      ui_sounds: state.config.ui_sounds,
      ui_language: state.config.ui_language,
    });
    renderTopbarAndSession();
    renderPlayView();
    renderProfileView();
    playMcSound("click");
    const lang = getEffectiveLanguage();
    statusEl.textContent = tr(lang, "profile.savedBadge");
    showToast(tr(lang, "profile.savedToast"), "success");
  } catch (err) {
    playMcSound("error");
    showToast(errorMessage(err), "error");
  }
}

async function saveAccessibilityPrefs(
  patch: {
    theme?: ThemeMode;
    ui_accent?: AccentTone;
    custom_accent_hex?: string | null;
    ui_corner?: CornerStyle;
    ui_layout?: LayoutMode;
    ui_compact?: boolean;
    ui_reduced_motion?: boolean;
    ui_sounds?: boolean;
    ui_language?: UiLanguage;
  },
  silentSound = false
): Promise<void> {
  if (!state) return;
  unlockAudioContext();
  if (!silentSound) {
    if (patch.ui_sounds !== undefined) {
      setSoundsEnabled(patch.ui_sounds);
      playMcSound(patch.ui_sounds ? "toggleOn" : "toggleOff", true);
    } else if (patch.ui_compact !== undefined) {
      playMcSound(patch.ui_compact ? "toggleOn" : "toggleOff");
    } else if (patch.ui_reduced_motion !== undefined) {
      playMcSound(patch.ui_reduced_motion ? "toggleOn" : "toggleOff");
    } else {
      playMcSound("click");
    }
  }
  const prevLang = getEffectiveLanguage();
  try {
    state.config = await ipc.saveProfileSettings({
      username: state.config.username,
      theme: patch.theme ?? state.config.theme,
      ui_accent: patch.ui_accent ?? state.config.ui_accent,
      custom_accent_hex:
        patch.custom_accent_hex !== undefined
          ? patch.custom_accent_hex
          : state.config.custom_accent_hex,
      ui_corner: patch.ui_corner ?? state.config.ui_corner,
      ui_layout: patch.ui_layout ?? state.config.ui_layout,
      ui_compact: patch.ui_compact ?? state.config.ui_compact,
      ui_reduced_motion: patch.ui_reduced_motion ?? state.config.ui_reduced_motion,
      ui_sounds: patch.ui_sounds ?? state.config.ui_sounds,
      ui_language: patch.ui_language ?? state.config.ui_language,
    });
    if (patch.ui_language !== undefined || getEffectiveLanguage() !== prevLang) {
      byId<HTMLSpanElement>("gfx-save-status").textContent = "";
      byId<HTMLSpanElement>("profile-save-status").textContent = "";
      renderAll();
      updateMaximizeIconState(lastMaximizedState);
    } else {
      applyPreferences();
      renderProfileView();
    }
  } catch (err) {
    playMcSound("error");
    showToast(errorMessage(err), "error");
  }
}

async function handlePickSkin(): Promise<void> {
  if (!state) return;
  const feedback = byId<HTMLDivElement>("skin-feedback");
  try {
    const res = await ipc.pickAndImportSkin();
    if (!res) {
      return;
    }
    state.skin_rgba = res.rgba;
    state.config.skin_file = res.path;
    state.config.skin_model = res.skin_model;
    state.skin_is_slim = res.skin_is_slim;
    renderTopbarAndSession();
    renderProfileView();
    setBanner(feedback, res.note, "success");
  } catch (err) {
    const msg = errorMessage(err);
    const low = msg.toLowerCase();
    if (!low.includes("iptal") && !low.includes("cancel") && !low.includes("abgebrochen")) {
      setBanner(feedback, msg, "error");
    }
  }
}

async function handleResetSkin(): Promise<void> {
  if (!state) return;
  const feedback = byId<HTMLDivElement>("skin-feedback");
  try {
    const res = await ipc.resetSkin();
    state.skin_rgba = res.rgba;
    state.config.skin_file = res.path;
    state.config.skin_model = res.skin_model;
    state.skin_is_slim = res.skin_is_slim;
    renderTopbarAndSession();
    renderProfileView();
    setBanner(feedback, res.note, "success");
  } catch (err) {
    setBanner(feedback, errorMessage(err), "error");
  }
}

async function handleSkinModelChange(model: SkinModel): Promise<void> {
  if (!state) return;
  const feedback = byId<HTMLDivElement>("skin-feedback");
  try {
    const res = await ipc.saveSkinModel(model);
    state.config.skin_model = res.skin_model;
    state.skin_is_slim = res.skin_is_slim;
    renderProfileView();
    const lang = getEffectiveLanguage();
    const label = tr(lang, res.skin_is_slim ? "skin.slimShort" : "skin.classicShort");
    setBanner(feedback, tr(lang, "skin.modelUpdated", { label }), "success");
  } catch (err) {
    setBanner(feedback, errorMessage(err), "error");
  }
}

async function handleAutoOptimize(): Promise<void> {
  if (!state) return;
  const btn = byId<HTMLButtonElement>("btn-auto-optimize");
  btn.disabled = true;
  try {
    const res = await ipc.autoOptimizeGraphics();
    state.config.graphics = res.graphics;
    state.hardware_notice = res.reason;
    byId<HTMLSpanElement>("gfx-save-status").textContent = "";
    renderPlayView();
    renderSystemView();
    showToast(tr(getEffectiveLanguage(), "system.autoOptToast"), "success");
  } catch (err) {
    showToast(errorMessage(err), "error");
  } finally {
    btn.disabled = false;
  }
}

async function handleSaveGraphics(e: SubmitEvent): Promise<void> {
  e.preventDefault();
  if (!state) return;
  const statusEl = byId<HTMLSpanElement>("gfx-save-status");
  const nextGfx: GraphicsSettings = {
    preset: "Custom",
    render_distance: Number(byId<HTMLInputElement>("gfx-render-dist").value) || 12,
    simulation_distance: Number(byId<HTMLInputElement>("gfx-sim-dist").value) || 10,
    fov_degrees: Number(byId<HTMLInputElement>("gfx-fov").value) || 75,
    vsync: byId<HTMLInputElement>("gfx-vsync").checked,
    max_fps: Number(byId<HTMLSelectElement>("gfx-max-fps").value) || 120,
    mouse_sensitivity: state.config.graphics.mouse_sensitivity,
    smooth_lighting: byId<HTMLInputElement>("gfx-ao").checked,
    fancy_graphics: byId<HTMLInputElement>("gfx-fancy").checked,
    jvm_ram_mb: Number(byId<HTMLSelectElement>("gfx-jvm-ram").value) || 2048,
    jvm_gc: byId<HTMLSelectElement>("gfx-jvm-gc").value === "ZGC" ? "ZGC" : "G1GC",
    show_fps_hud: state.config.graphics.show_fps_hud,
    auto_configured: false,
  };

  try {
    state.config = await ipc.saveGraphicsSettings(nextGfx);
    renderPlayView();
    renderSystemView();
    const lang = getEffectiveLanguage();
    statusEl.textContent = tr(lang, "system.gfxSavedInline");
    showToast(tr(lang, "system.gfxSavedToast"), "success");
  } catch (err) {
    showToast(errorMessage(err), "error");
  }
}

async function handleAddServer(e: SubmitEvent): Promise<void> {
  e.preventDefault();
  if (!state) return;
  const nameInput = byId<HTMLInputElement>("srv-name");
  const addrInput = byId<HTMLInputElement>("srv-addr");
  const errEl = byId<HTMLParagraphElement>("server-form-error");

  const addr = addrInput.value.trim();
  if (!addr) {
    errEl.textContent = tr(getEffectiveLanguage(), "servers.addrEmpty");
    errEl.hidden = false;
    addrInput.focus();
    return;
  }
  errEl.hidden = true;

  try {
    state.config.saved_servers = await ipc.addServer(nameInput.value.trim(), addr);
    renderServersList();
    showToast(tr(getEffectiveLanguage(), "servers.addedToast"), "success");
  } catch (err) {
    errEl.textContent = errorMessage(err);
    errEl.hidden = false;
  }
}

async function handleRemoveServer(index: number): Promise<void> {
  if (!state) return;
  try {
    state.config.saved_servers = await ipc.removeServer(index);
    renderServersList();
  } catch (err) {
    showToast(errorMessage(err), "error");
  }
}

async function handlePingServer(address: string): Promise<void> {
  serverChecks.set(address, { status: "checking" });
  renderServersList();
  try {
    const ping = await ipc.pingServerCmd(address);
    serverChecks.set(address, { status: "online", ping });
  } catch (err) {
    serverChecks.set(address, { status: "error", message: errorMessage(err) });
  }
  renderServersList();
}

async function handleProbeServer(address: string): Promise<void> {
  if (!state) return;
  const probeBox = byId<HTMLDivElement>("probe-feedback");
  const lang = getEffectiveLanguage();
  setBanner(probeBox, tr(lang, "servers.probing", { addr: address }), "neutral");
  try {
    const res = await ipc.probeLocalServerCmd(address, state.config.username);
    if (res.kind === "ReadyInPlay") {
      setBanner(
        probeBox,
        tr(lang, "servers.probeReady", { addr: address, info: res.data.server_info }),
        "success"
      );
    } else if (res.kind === "RequiresOfficialAuth") {
      setBanner(probeBox, tr(lang, "servers.probeAuth", { addr: address }), "error");
    } else {
      setBanner(probeBox, tr(lang, "servers.probeDisc", { addr: address, reason: res.data }), "error");
    }
  } catch (err) {
    setBanner(probeBox, `${address}: ${errorMessage(err)}`, "error");
  }
}

let deckRafId: number | null = null;
let deckPrevButtons: boolean[] = [];
let deckLastRepeatDir: "up" | "down" | "left" | "right" | null = null;
let deckLastRepeatAt = 0;

function getVisibleDeckFocusables(): HTMLElement[] {
  const nodes = Array.from(
    document.querySelectorAll<HTMLElement>(
      ".app-shell button:not([disabled]), .app-shell input:not([disabled]), .app-shell select:not([disabled]), .app-shell [tabindex]:not([tabindex='-1'])"
    )
  );
  return nodes.filter((el) => {
    if (el.hidden || el.closest("[hidden]")) return false;
    const rect = el.getBoundingClientRect();
    return rect.width > 0 && rect.height > 0 && el.offsetParent !== null;
  });
}

function moveDeckFocus(dir: "up" | "down" | "left" | "right"): void {
  const isPlayVisible = !byId<HTMLElement>("view-play").hidden;
  const active = document.activeElement as HTMLElement | null;

  if (getEffectiveLayout() === "DeckMode" && isPlayVisible && deckCards.length > 0) {
    if (active?.classList.contains("deck-card")) {
      if (dir === "left") {
        selectDeckCard(selectedDeckIdx - 1, { scroll: true, focusBtn: true, withSound: true });
        return;
      }
      if (dir === "right") {
        selectDeckCard(selectedDeckIdx + 1, { scroll: true, focusBtn: true, withSound: true });
        return;
      }
      if (dir === "up") {
        const primary = byId<HTMLButtonElement>("btn-deck-primary");
        const secondary = byId<HTMLButtonElement>("btn-deck-secondary");
        const target = !primary.disabled ? primary : secondary;
        target.focus({ preventScroll: true });
        playMcSound("focus");
        return;
      }
    } else if (active?.closest(".console-hero-actions") && dir === "down") {
      selectDeckCard(selectedDeckIdx, { scroll: true, focusBtn: true, withSound: false });
      playMcSound("focus");
      return;
    } else if (!active || active === document.body) {
      if (dir === "left") {
        selectDeckCard(selectedDeckIdx - 1, { scroll: true, focusBtn: true, withSound: true });
        return;
      }
      if (dir === "right") {
        selectDeckCard(selectedDeckIdx + 1, { scroll: true, focusBtn: true, withSound: true });
        return;
      }
    }
  }

  const candidates = getVisibleDeckFocusables();
  if (candidates.length === 0) return;

  if (!active || !candidates.includes(active)) {
    const preferred =
      candidates.find((el) => el.id === "btn-deck-primary" && !el.hidden) ??
      candidates.find((el) => el.classList.contains("btn-launch")) ??
      candidates.find((el) => el.classList.contains("active")) ??
      candidates[0];
    preferred.focus({ preventScroll: true });
    preferred.scrollIntoView({ block: "nearest", inline: "nearest" });
    playMcSound("focus");
    return;
  }

  const cur = active.getBoundingClientRect();
  const cx = cur.left + cur.width / 2;
  const cy = cur.top + cur.height / 2;

  let best: HTMLElement | null = null;
  let bestScore = Number.POSITIVE_INFINITY;

  for (const el of candidates) {
    if (el === active) continue;
    const r = el.getBoundingClientRect();
    const tx = r.left + r.width / 2;
    const ty = r.top + r.height / 2;
    const dx = tx - cx;
    const dy = ty - cy;

    let primary = 0;
    let ortho = 0;
    if (dir === "right") {
      if (dx <= 6) continue;
      primary = dx;
      ortho = Math.abs(dy);
    } else if (dir === "left") {
      if (dx >= -6) continue;
      primary = -dx;
      ortho = Math.abs(dy);
    } else if (dir === "down") {
      if (dy <= 6) continue;
      primary = dy;
      ortho = Math.abs(dx);
    } else {
      if (dy >= -6) continue;
      primary = -dy;
      ortho = Math.abs(dx);
    }

    const score = primary + ortho * 2.4;
    if (score < bestScore) {
      bestScore = score;
      best = el;
    }
  }

  if (best) {
    best.focus({ preventScroll: true });
    best.scrollIntoView({ block: "nearest", inline: "nearest" });
    playMcSound("focus");
  }
}

function cycleActiveView(delta: -1 | 1): void {
  const activeBtn = document.querySelector<HTMLButtonElement>(".nav-btn.active[data-nav]");
  const current = (activeBtn?.dataset.nav as ViewId | undefined) ?? "play";
  const idx = VIEW_ORDER.indexOf(current);
  const nextIdx = (idx + delta + VIEW_ORDER.length) % VIEW_ORDER.length;
  setActiveView(VIEW_ORDER[nextIdx], true, true);
}

function exitConsoleMode(): void {
  if (getEffectiveLayout() !== "DeckMode") return;
  unlockAudioContext();
  playMcSound("back", true);
  consoleModeActive = false;
  consoleAudioMuted = false;

  const url = new URL(window.location.href);
  if (url.searchParams.has("layout")) {
    url.searchParams.delete("layout");
    window.history.replaceState(null, "", url.toString());
  }

  if (state && state.config.ui_layout === "DeckMode") {
    void saveAccessibilityPrefs({ ui_layout: lastDesktopLayout }, true);
  } else {
    applyPreferences();
    renderProfileView();
  }
  void ipc.setFullscreen(false).catch(() => undefined);
}

function pollGamepadFrame(): void {
  if (getEffectiveLayout() !== "DeckMode") {
    deckRafId = null;
    return;
  }

  const badge = document.getElementById("deck-gamepad-status");
  const label = document.getElementById("deck-gamepad-label");
  const pads = navigator.getGamepads ? Array.from(navigator.getGamepads()) : [];
  const pad = pads.find((p): p is Gamepad => p !== null && p.connected);

  if (badge && label) {
    const lang = getEffectiveLanguage();
    badge.classList.toggle("is-connected", Boolean(pad));
    label.textContent = pad
      ? pad.id.replace(/\s*\(.*?\)/g, "").trim().slice(0, 28) || tr(lang, "deck.gamepadConnected")
      : tr(lang, "deck.gamepadWaiting");
  }

  if (pad) {
    const btns = pad.buttons.map((b) => b.pressed);
    const pressedEdge = (i: number) => Boolean(btns[i] && !deckPrevButtons[i]);

    const ax = pad.axes[0] ?? 0;
    const ay = pad.axes[1] ?? 0;
    let dir: "up" | "down" | "left" | "right" | null = null;
    if (btns[12] || ay < -0.55) dir = "up";
    else if (btns[13] || ay > 0.55) dir = "down";
    else if (btns[14] || ax < -0.55) dir = "left";
    else if (btns[15] || ax > 0.55) dir = "right";

    const now = performance.now();
    if (dir && (dir !== deckLastRepeatDir || now - deckLastRepeatAt > 190)) {
      deckLastRepeatDir = dir;
      deckLastRepeatAt = now;
      unlockAudioContext();
      moveDeckFocus(dir);
    } else if (!dir) {
      deckLastRepeatDir = null;
    }

    if (pressedEdge(0)) {
      unlockAudioContext();
      const active = document.activeElement as HTMLElement | null;
      const target =
        active && active !== document.body && active !== document.documentElement
          ? active
          : byId<HTMLElement>("btn-deck-primary");
      target.click();
    }
    if (pressedEdge(1)) {
      unlockAudioContext();
      const openDialog = document.querySelector<HTMLDialogElement>("dialog[open]");
      if (openDialog) {
        playMcSound("back");
        openDialog.close();
      } else {
        exitConsoleMode();
      }
    }
    if (pressedEdge(3)) {
      toggleUiSound();
    }
    if (pressedEdge(4)) {
      unlockAudioContext();
      cycleActiveView(-1);
    }
    if (pressedEdge(5)) {
      unlockAudioContext();
      cycleActiveView(1);
    }

    deckPrevButtons = btns;
  }

  deckRafId = window.requestAnimationFrame(pollGamepadFrame);
}

function syncDeckGamepadLoop(active: boolean): void {
  if (active && deckRafId === null) {
    deckRafId = window.requestAnimationFrame(pollGamepadFrame);
  } else if (!active && deckRafId !== null) {
    window.cancelAnimationFrame(deckRafId);
    deckRafId = null;
    deckPrevButtons = [];
    deckLastRepeatDir = null;
  }
}

function initDeckKeyboardNav(): void {
  window.addEventListener("gamepadconnected", () => {
    if (getEffectiveLayout() === "DeckMode") {
      syncDeckGamepadLoop(true);
    }
  });

  window.addEventListener("keydown", (e) => {
    unlockAudioContext();
    if (getEffectiveLayout() !== "DeckMode") return;

    const target = e.target as HTMLElement | null;
    const isTextControl =
      target instanceof HTMLTextAreaElement ||
      target instanceof HTMLSelectElement ||
      (target instanceof HTMLInputElement && target.type !== "checkbox" && target.type !== "button");

    if (e.key === "Escape") {
      if (!document.querySelector("dialog[open]") && !(target instanceof HTMLSelectElement)) {
        e.preventDefault();
        exitConsoleMode();
      }
      return;
    }

    if (isTextControl) return;

    if (e.key === "ArrowUp") {
      e.preventDefault();
      moveDeckFocus("up");
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      moveDeckFocus("down");
    } else if (e.key === "ArrowLeft") {
      e.preventDefault();
      moveDeckFocus("left");
    } else if (e.key === "ArrowRight") {
      e.preventDefault();
      moveDeckFocus("right");
    } else if (e.key === "q" || e.key === "Q") {
      e.preventDefault();
      cycleActiveView(-1);
    } else if (e.key === "e" || e.key === "E") {
      e.preventDefault();
      cycleActiveView(1);
    } else if (e.key === "m" || e.key === "M") {
      e.preventDefault();
      toggleUiSound();
    }
  });
}

let lastMaximizedState = false;

function updateMaximizeIconState(maximized: boolean): void {
  lastMaximizedState = maximized;
  const lang = getEffectiveLanguage();
  const maxUse = document.getElementById("ico-win-max-use");
  const maxBtn = document.getElementById("btn-win-maximize");
  if (maxUse) {
    maxUse.setAttribute("href", maximized ? "#ico-win-restore" : "#ico-win-max");
  }
  if (maxBtn) {
    maxBtn.setAttribute("aria-label", tr(lang, maximized ? "win.restore" : "win.maximize"));
    maxBtn.setAttribute("title", tr(lang, maximized ? "win.restoreTitle" : "win.maximizeTitle"));
  }
}

async function syncWindowMaximizeIcon(): Promise<void> {
  try {
    const maximized = await ipc.windowIsMaximized();
    updateMaximizeIconState(maximized);
  } catch {
    // Headless preview fallback
  }
}

function wireWindowControls(): void {
  const titlebar = document.getElementById("app-titlebar");
  const minBtn = document.getElementById("btn-win-minimize");
  const maxBtn = document.getElementById("btn-win-maximize");
  const closeBtn = document.getElementById("btn-win-close");

  minBtn?.addEventListener("click", () => {
    unlockAudioContext();
    playMcSound("click");
    void ipc.windowMinimize();
  });

  maxBtn?.addEventListener("click", () => {
    unlockAudioContext();
    playMcSound("click");
    void ipc.windowToggleMaximize().then(updateMaximizeIconState).catch(() => undefined);
  });

  closeBtn?.addEventListener("click", () => {
    unlockAudioContext();
    playMcSound("back");
    void ipc.windowClose();
  });

  titlebar?.addEventListener("dblclick", (e) => {
    const target = e.target as HTMLElement | null;
    if (target?.closest(".titlebar-actions")) return;
    void ipc.windowToggleMaximize().then(updateMaximizeIconState).catch(() => undefined);
  });

  titlebar?.addEventListener("mousedown", (e) => {
    if (e.button !== 0 || e.detail > 1) return;
    const target = e.target as HTMLElement | null;
    if (target?.closest("button, a, input, select, .titlebar-actions")) return;
    void ipc.windowStartDrag().catch(() => undefined);
  });

  window.addEventListener("resize", () => {
    void syncWindowMaximizeIcon();
  });
}

function wireEvents(): void {
  wireWindowControls();
  const unlockOnce = () => unlockAudioContext();
  window.addEventListener("pointerdown", unlockOnce, { passive: true });
  window.addEventListener("keydown", unlockOnce, { passive: true });

  const navButtons = document.querySelectorAll<HTMLButtonElement>(".nav-btn[data-nav]");
  for (const btn of Array.from(navButtons)) {
    btn.addEventListener("click", () => {
      const target = btn.dataset.nav as ViewId | undefined;
      if (target) setActiveView(target, true, true);
    });
  }

  window.addEventListener("hashchange", () => {
    setActiveView(readInitialViewFromHash(), false);
  });

  byId<HTMLButtonElement>("btn-rail-theme").addEventListener("click", () => {
    if (!state) return;
    const idx = THEME_ORDER.indexOf(state.config.theme);
    const next = THEME_ORDER[(idx + 1) % THEME_ORDER.length];
    void saveAccessibilityPrefs({ theme: next });
  });

  byId<HTMLButtonElement>("btn-rail-layout").addEventListener("click", () => {
    if (!state) return;
    if (getEffectiveLayout() === "DeckMode") {
      exitConsoleMode();
      return;
    }
    const idx = DESKTOP_LAYOUT_ORDER.indexOf(state.config.ui_layout);
    const next = DESKTOP_LAYOUT_ORDER[(idx + 1 + DESKTOP_LAYOUT_ORDER.length) % DESKTOP_LAYOUT_ORDER.length];
    void saveAccessibilityPrefs({ ui_layout: next });
  });

  byId<HTMLButtonElement>("btn-rail-sound").addEventListener("click", () => {
    toggleUiSound();
  });

  byId<HTMLButtonElement>("btn-enter-deck").addEventListener("click", () => {
    enterConsoleMode();
  });

  byId<HTMLButtonElement>("btn-exit-deck").addEventListener("click", () => {
    exitConsoleMode();
  });

  byId<HTMLButtonElement>("btn-deck-primary").addEventListener("click", () => {
    const card = deckCards[selectedDeckIdx];
    if (card) handleDeckPrimaryAction(card);
  });

  byId<HTMLButtonElement>("btn-deck-secondary").addEventListener("click", () => {
    const card = deckCards[selectedDeckIdx];
    if (card) handleDeckSecondaryAction(card);
  });

  byId<HTMLButtonElement>("btn-deck-prepare").addEventListener("click", () => {
    unlockAudioContext();
    void handlePrepareVanilla();
  });

  byId<HTMLButtonElement>("btn-deck-prev").addEventListener("click", () => {
    unlockAudioContext();
    selectDeckCard(selectedDeckIdx - 1, { scroll: true, focusBtn: false, withSound: true });
  });

  byId<HTMLButtonElement>("btn-deck-next").addEventListener("click", () => {
    unlockAudioContext();
    selectDeckCard(selectedDeckIdx + 1, { scroll: true, focusBtn: false, withSound: true });
  });

  const deckCarousel = byId<HTMLDivElement>("deck-carousel");
  deckCarousel.addEventListener(
    "wheel",
    (e) => {
      if (Math.abs(e.deltaY) > Math.abs(e.deltaX)) {
        deckCarousel.scrollLeft += e.deltaY;
        e.preventDefault();
      }
    },
    { passive: false }
  );

  byId<HTMLButtonElement>("btn-goto-worlds").addEventListener("click", () => {
    setActiveView("worlds", true, true);
  });

  byId<HTMLSelectElement>("play-save-select").addEventListener("change", (e) => {
    const val = (e.currentTarget as HTMLSelectElement).value;
    void handleSelectSave(val ? val : null);
  });

  byId<HTMLButtonElement>("btn-prepare-gradle").addEventListener("click", () => {
    void handlePrepareVanilla();
  });

  byId<HTMLButtonElement>("btn-install-java").addEventListener("click", () => {
    void handleInstallJava21();
  });

  byId<HTMLButtonElement>("btn-recheck-java").addEventListener("click", () => {
    void handleRecheckJava21();
  });

  byId<HTMLButtonElement>("btn-open-java-site").addEventListener("click", () => {
    void handleOpenJavaPage();
  });

  byId<HTMLButtonElement>("btn-system-install-java").addEventListener("click", () => {
    void handleInstallJava21();
  });

  byId<HTMLButtonElement>("btn-system-recheck-java").addEventListener("click", () => {
    void handleRecheckJava21();
  });

  byId<HTMLButtonElement>("btn-system-open-java").addEventListener("click", () => {
    void handleOpenJavaPage();
  });

  byId<HTMLButtonElement>("btn-launch-client").addEventListener("click", () => {
    if (!state) return;
    const saveId = state.config.last_save_id;
    void handleLaunchClient(saveId ? "singleplayer" : "title", saveId, null);
  });

  byId<HTMLButtonElement>("btn-refresh-worlds").addEventListener("click", () => {
    void handleRefreshSaves();
  });

  byId<HTMLButtonElement>("btn-open-mc-for-world").addEventListener("click", () => {
    void handleLaunchClient("title", null, null);
  });

  byId<HTMLFormElement>("profile-form").addEventListener("submit", (e) => {
    void handleSaveProfileForm(e);
  });

  for (const btn of Array.from(document.querySelectorAll<HTMLButtonElement>("[data-theme-choice]"))) {
    btn.addEventListener("click", () => {
      const choice = btn.dataset.themeChoice as ThemeMode | undefined;
      if (choice && THEME_ORDER.includes(choice)) {
        void saveAccessibilityPrefs({ theme: choice });
      }
    });
  }

  for (const btn of Array.from(document.querySelectorAll<HTMLButtonElement>("[data-layout-choice]"))) {
    btn.addEventListener("click", () => {
      const choice = btn.dataset.layoutChoice as LayoutMode | undefined;
      if (!choice || !LAYOUT_ORDER.includes(choice)) return;
      if (choice === "DeckMode") {
        enterConsoleMode();
        return;
      }
      if (getEffectiveLayout() === "DeckMode") {
        consoleModeActive = false;
        consoleAudioMuted = false;
        playMcSound("back", true);
        void ipc.setFullscreen(false).catch(() => undefined);
      }
      void saveAccessibilityPrefs({ ui_layout: choice });
    });
  }

  for (const btn of Array.from(document.querySelectorAll<HTMLButtonElement>("[data-accent-choice]"))) {
    btn.addEventListener("click", () => {
      const choice = btn.dataset.accentChoice as AccentTone | undefined;
      if (choice && ACCENT_ORDER.includes(choice)) {
        void saveAccessibilityPrefs({ ui_accent: choice });
      }
    });
  }

  for (const btn of Array.from(document.querySelectorAll<HTMLButtonElement>("[data-corner-choice]"))) {
    btn.addEventListener("click", () => {
      const choice = btn.dataset.cornerChoice as CornerStyle | undefined;
      if (choice && CORNER_ORDER.includes(choice)) {
        void saveAccessibilityPrefs({ ui_corner: choice });
      }
    });
  }

  for (const btn of Array.from(document.querySelectorAll<HTMLButtonElement>("[data-lang-choice]"))) {
    btn.addEventListener("click", () => {
      const choice = btn.dataset.langChoice as UiLanguage | undefined;
      if (choice && UI_LANGUAGE_ORDER.includes(choice)) {
        const url = new URL(window.location.href);
        if (url.searchParams.has("lang")) {
          url.searchParams.delete("lang");
          window.history.replaceState(null, "", url.toString());
        }
        void saveAccessibilityPrefs({ ui_language: choice });
      }
    });
  }

  const customColorInput = byId<HTMLInputElement>("pref-custom-accent");
  customColorInput.addEventListener("input", () => {
    if (!state) return;
    const valid = normalizeHex(customColorInput.value);
    if (valid) {
      byId<HTMLSpanElement>("pref-custom-hex-label").textContent = valid;
      byId<HTMLLabelElement>("pref-custom-chip").classList.add("is-active");
      applyDynamicDesignTokens(getEffectiveTheme(), "Custom", valid, getEffectiveCorner());
    }
  });
  customColorInput.addEventListener("change", () => {
    const valid = normalizeHex(customColorInput.value);
    if (valid) {
      void saveAccessibilityPrefs({ ui_accent: "Custom", custom_accent_hex: valid });
    }
  });

  byId<HTMLInputElement>("pref-compact").addEventListener("change", (e) => {
    void saveAccessibilityPrefs({ ui_compact: (e.currentTarget as HTMLInputElement).checked });
  });

  byId<HTMLInputElement>("pref-reduced-motion").addEventListener("change", (e) => {
    void saveAccessibilityPrefs({ ui_reduced_motion: (e.currentTarget as HTMLInputElement).checked });
  });

  byId<HTMLInputElement>("pref-sounds").addEventListener("change", (e) => {
    toggleUiSound((e.currentTarget as HTMLInputElement).checked);
  });

  const skinModeBtns = document.querySelectorAll<HTMLButtonElement>("[data-skin-view]");
  for (const btn of Array.from(skinModeBtns)) {
    btn.addEventListener("click", () => {
      const mode = btn.dataset.skinView as SkinViewMode | undefined;
      if (mode && state) {
        skinViewMode = mode;
        playMcSound("click");
        renderProfileView();
      }
    });
  }

  byId<HTMLButtonElement>("btn-pick-skin").addEventListener("click", () => {
    void handlePickSkin();
  });

  byId<HTMLButtonElement>("btn-reset-skin").addEventListener("click", () => {
    void handleResetSkin();
  });

  byId<HTMLSelectElement>("skin-model-select").addEventListener("change", (e) => {
    const raw = (e.currentTarget as HTMLSelectElement).value;
    const model: SkinModel = raw === "Slim" || raw === "Classic" ? raw : "Auto";
    void handleSkinModelChange(model);
  });

  byId<HTMLButtonElement>("btn-auto-optimize").addEventListener("click", () => {
    void handleAutoOptimize();
  });

  byId<HTMLInputElement>("gfx-render-dist").addEventListener("input", (e) => {
    byId<HTMLOutputElement>("gfx-render-dist-out").textContent = (e.currentTarget as HTMLInputElement).value;
  });

  byId<HTMLInputElement>("gfx-sim-dist").addEventListener("input", (e) => {
    byId<HTMLOutputElement>("gfx-sim-dist-out").textContent = (e.currentTarget as HTMLInputElement).value;
  });

  byId<HTMLInputElement>("gfx-fov").addEventListener("input", (e) => {
    byId<HTMLOutputElement>("gfx-fov-out").textContent = `${(e.currentTarget as HTMLInputElement).value}°`;
  });

  byId<HTMLFormElement>("graphics-form").addEventListener("submit", (e) => {
    void handleSaveGraphics(e);
  });

  byId<HTMLFormElement>("add-server-form").addEventListener("submit", (e) => {
    void handleAddServer(e);
  });

  const dialog = byId<HTMLDialogElement>("delete-world-dialog");
  const closeDialog = () => {
    if (dialog.open) dialog.close();
  };
  byId<HTMLButtonElement>("delete-dialog-close").addEventListener("click", closeDialog);
  byId<HTMLButtonElement>("delete-dialog-cancel").addEventListener("click", closeDialog);
  byId<HTMLButtonElement>("delete-dialog-confirm").addEventListener("click", () => {
    void confirmDeleteSave();
  });
  dialog.addEventListener("close", () => {
    const trigger = lastDialogTrigger;
    lastDialogTrigger = null;
    if (trigger && trigger.isConnected) {
      trigger.focus();
    } else {
      byId<HTMLButtonElement>("btn-refresh-worlds").focus();
    }
  });
}

async function init(): Promise<void> {
  wireEvents();
  initDeckKeyboardNav();
  initMinecraftSounds();
  setActiveView(readInitialViewFromHash(), false);

  try {
    state = await ipc.getBootstrapState();
    renderAll();
  } catch (err) {
    setBanner(
      byId<HTMLDivElement>("launch-feedback"),
      tr(getEffectiveLanguage(), "err.bootstrap", { err: errorMessage(err) }),
      "error"
    );
  }

  await ipc.onLaunchLog((payload) => {
    appendLogLine(payload.line, payload.is_err);
  });

  await ipc.onSessionEnded((payload) => {
    if (!state) return;
    state.running_session = null;
    state.saves = payload.saves;
    state.vanilla = payload.vanilla;
    renderTopbarAndSession();
    renderPlayView();
    renderWorldsView();
    renderSystemView();

    const lang = getEffectiveLanguage();
    const feedback = byId<HTMLDivElement>("launch-feedback");
    if (payload.error) {
      playMcSound("error");
      setBanner(feedback, tr(lang, "err.sessionFailed", { err: payload.error }), "error");
      showToast(payload.error, "error");
    } else if (payload.exit_code !== null && payload.exit_code !== 0) {
      playMcSound("error");
      const msg = tr(lang, "err.gradleExit", { code: payload.exit_code });
      setBanner(feedback, msg, "error");
      showToast(msg, "error");
    } else if (payload.kind === "java-install") {
      if (payload.vanilla.java_ready) {
        playMcSound("launch");
        const msg = tr(lang, "ok.javaInstalled", { ver: payload.vanilla.java_version_label });
        setBanner(feedback, msg, "success");
        showToast(msg, "success");
      } else {
        playMcSound("error");
        const msg = tr(lang, "err.javaNotVerified");
        setBanner(feedback, msg, "error");
        showToast(msg, "error");
      }
    } else if (payload.kind === "prepare") {
      playMcSound("launch");
      const msg = tr(lang, "ok.prebuildDone");
      setBanner(feedback, msg, "success");
      showToast(msg, "success");
    } else {
      playMcSound("click");
      const msg = tr(lang, "ok.sessionClosed");
      setBanner(feedback, msg, "success");
      showToast(msg, "success");
    }
  });
}

void init();
