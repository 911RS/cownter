export type Status = "live" | "polling" | "stale" | "error" | "noLink";

export type Pick =
  | { kind: "jsonPath"; value: string }
  | { kind: "firstNumber" }
  | { kind: "regex"; value: string };

export type SecretOrText = { kind: "text"; value: string } | { kind: "secret"; value: string };

export interface Header {
  name: string;
  value: SecretOrText;
}

export type YouTubeMetric = "subscribers" | "views" | "videos";
export type GitHubMetric = "stars" | "forks" | "openIssues" | "watchers" | "followers" | "publicRepos";

export type SourceCfg =
  | { type: "customUrl"; url: string; method: string; headers: Header[]; body: string | null; pick: Pick }
  | { type: "youtube"; channelId: string; metric: YouTubeMetric; apiKey: string }
  | { type: "github"; target: string; metric: GitHubMetric; token: string | null }
  | { type: "shopify"; shop: string; token: string }
  | { type: "stripe"; apiKey: string; currency: string }
  | { type: "formula"; expr: string };

export type SourceType = SourceCfg["type"];

export interface Icon {
  kind: "emoji" | "builtin" | "image" | "favicon";
  value: string;
  bg: string;
}

export interface NumberFormat {
  style: "full" | "compact";
  decimals: number | null;
  prefix: string;
  suffix: string;
}

export interface Sounds {
  onChange: string | null;
  onGoal: string | null;
}

export interface Entry {
  id: number;
  slug: string;
  name: string;
  icon: Icon;
  groupId: number | null;
  source: SourceCfg;
  intervalS: number;
  format: NumberFormat;
  goalValue: number | null;
  celebrate: boolean;
  sounds: Sounds;
  sort: number;
  textMode?: boolean;
  goalText?: string | null;
}

export interface EntryView {
  entry: Entry;
  value: number | null;
  prev: number | null;
  status: Status;
  error: string | null;
  updatedAt: number | null;
  deltaToday: number | null;
  spark: [number, number][];
  goalProgress: number | null;
  goalEta?: number | null;
  streak?: number;
  text?: string | null;
  prevText?: string | null;
}

export type AlertKind = "above" | "below" | "changePct" | "riseBy" | "dropBy";
export type AlertEffect = "" | "float" | "pop" | "burst" | "confetti" | "shake" | "glow" | "flash" | "zoom" | "rain";

export interface Alert {
  id: number;
  entryId: number;
  kind: AlertKind;
  threshold: number;
  windowS: number;
  enabled: boolean;
  notify: boolean;
  effect: AlertEffect;
  lastFiredAt: number | null;
}

export interface Group {
  id: number;
  name: string;
  sort: number;
}

export type CoreEvent =
  | { type: "entryUpdated"; view: EntryView }
  | { type: "entryRemoved"; entryId: number }
  | { type: "valueChanged"; entryId: number; value: number; prev: number }
  | { type: "alertFired"; entryId: number; alertId: number; message: string; value: number; diff: number | null; notify: boolean; effect: AlertEffect }
  | { type: "textChanged"; entryId: number; text: string; prev: string }
  | { type: "goalReached"; entryId: number; value: number; text?: string | null }
  | { type: "allTimeHigh"; entryId: number; value: number }
  | { type: "authFailed"; entryId: number; message: string };

export type SkinName =
  | "clean"
  | "dmsans"
  | "manrope"
  | "poppins"
  | "montserrat"
  | "rubik"
  | "lexend"
  | "geometric"
  | "modern"
  | "rounded"
  | "grotesk"
  | "tall"
  | "barlow"
  | "poster"
  | "wide"
  | "future"
  | "mono"
  | "plexmono"
  | "spacemono"
  | "dotMatrix"
  | "calculator"
  | "neon";
export type Tint = "white" | "chroma" | "amber" | "phosphor" | "ice" | "ruby" | "violet" | "classic";
export type Size = "xs" | "s" | "m" | "l" | "xl";

export interface FormCfg {
  enabled: boolean;
  skin: SkinName;
  tint: Tint;
  size: Size;
  groupId: number | null;
  entryIds: number[];
  side: "left" | "right";
  offset: number;
  position: Position;
  cycleS: number;
  offsetX: number;
  offsetY: number;
  monitor: string;
  scale: number;
  opacity: number;
  bg: string;
  bgAlpha: number;
  roundness: number;
  shadow: boolean;
  showIcons: boolean;
  iconStyle?: IconStyle;
  spacing?: number;
  gap?: number;
  showDelta: boolean;
  showDots: boolean;
  showNames: boolean;
  clickOpens: boolean;
  hoverExpands: boolean;
  skinOpts?: Record<string, boolean>;
  formOpts?: Record<string, boolean>;
}

export type Position = "topLeft" | "topCenter" | "topRight" | "middleLeft" | "middleCenter" | "middleRight" | "bottomLeft" | "bottomCenter" | "bottomRight";

export interface DisplayConfig {
  version: number;
  island: FormCfg;
  edge: FormCfg;
  pop: FormCfg;
  forms: Record<string, FormCfg>;
  celebration: boolean;
  tray: { showNumber: boolean; entryId: number | null };
  volume: number;
  muted: boolean;
  hotkeyToggle: string;
  hotkeyRefresh: string;
  hotkeyPeek: string;
  launchAtLogin: boolean;
  hidden: boolean;
  onboarded: boolean;
}

export interface TestResult {
  status: number;
  elapsedMs: number;
  contentType: string | null;
  body: string;
  truncated: boolean;
  value: number | null;
  error: string | null;
  text?: string | null;
}

export type IconStyle = "tile" | "big" | "fade" | "glow" | "glyph" | "bar";
