import { useEffect, useMemo, useState } from "react";
import { api, errorText } from "../core/api";
import { formatValue, goalLeft, intervalLabel, slugify } from "../core/format";
import { SOUNDS, playSound } from "../core/sounds";
import type { Entry, EntryView, Group, Header, Icon, Pick, SourceCfg, SourceType, TestResult } from "../core/types";
import { EntryIcon } from "../components/EntryIcon";
import { AlertsEditor } from "./AlertsEditor";
import { IconPicker } from "./IconPicker";
import { SmartPicker } from "./SmartPicker";
import { Banner, Field, NumberInput, Segmented, Toggle } from "./ui";

const INTERVALS = [10, 30, 60, 300, 900, 1800, 3600, 21600, 86400];

const TYPES: { id: SourceType; label: string; hint: string; icon: Icon }[] = [
  { id: "customUrl", label: "Any URL", hint: "Your own API or any JSON endpoint", icon: { kind: "builtin", value: "globe", bg: "#22262d" } },
  { id: "youtube", label: "YouTube", hint: "Subscribers, views, videos", icon: { kind: "builtin", value: "play", bg: "#ff2d2d" } },
  { id: "github", label: "GitHub", hint: "Stars, forks, followers", icon: { kind: "builtin", value: "star", bg: "#24292f" } },
  { id: "shopify", label: "Shopify", hint: "Orders today", icon: { kind: "builtin", value: "cart", bg: "#3fa34d" } },
  { id: "stripe", label: "Stripe", hint: "Revenue today", icon: { kind: "builtin", value: "euro", bg: "#635bff" } },
  { id: "plausible", label: "Plausible", hint: "Site visitors", icon: { kind: "builtin", value: "eye", bg: "#5850ec" } },
  { id: "npm", label: "npm", hint: "Weekly downloads", icon: { kind: "builtin", value: "download", bg: "#cb3837" } },
  { id: "gumroad", label: "Gumroad", hint: "Sales, revenue", icon: { kind: "builtin", value: "tag", bg: "#ff90e8" } },
  { id: "lemonSqueezy", label: "Lemon Squeezy", hint: "Sales, revenue", icon: { kind: "builtin", value: "bag", bg: "#ffc233" } },
  { id: "productHunt", label: "Product Hunt", hint: "Upvotes on a post", icon: { kind: "builtin", value: "rocket", bg: "#da552f" } },
  { id: "discord", label: "Discord", hint: "Members, online now", icon: { kind: "builtin", value: "chat", bg: "#5865f2" } },
  { id: "formula", label: "Formula", hint: "Math on other numbers", icon: { kind: "builtin", value: "chart", bg: "#7a5af8" } },
  { id: "push", label: "Push", hint: "Sent by your scripts or the CLI", icon: { kind: "builtin", value: "bolt", bg: "#e8862b" } },
];

const TEXT_TYPES: SourceType[] = ["customUrl", "push"];
const KEYLESS_TYPES: SourceType[] = ["formula", "github", "push", "npm", "discord"];

const DEFAULT_NAMES: Record<SourceType, string> = {
  customUrl: "",
  youtube: "YouTube subscribers",
  github: "GitHub stars",
  shopify: "Orders today",
  stripe: "Revenue today",
  plausible: "Visitors today",
  npm: "npm downloads",
  gumroad: "Gumroad sales",
  lemonSqueezy: "Lemon Squeezy sales",
  productHunt: "Product Hunt upvotes",
  discord: "Discord members",
  formula: "",
  push: "",
};

function rid() {
  return Math.random().toString(36).slice(2, 8);
}

function defaultSource(t: SourceType, base: string): SourceCfg {
  switch (t) {
    case "customUrl":
      return { type: "customUrl", url: "", method: "GET", headers: [], body: null, pick: { kind: "jsonPath", value: "" } };
    case "youtube":
      return { type: "youtube", channelId: "", metric: "subscribers", apiKey: `${base}-apiKey` };
    case "github":
      return { type: "github", target: "", metric: "stars", token: null };
    case "shopify":
      return { type: "shopify", shop: "", token: `${base}-token` };
    case "stripe":
      return { type: "stripe", apiKey: `${base}-apiKey`, currency: "EUR" };
    case "plausible":
      return { type: "plausible", siteId: "", metric: "visitorsToday", apiKey: `${base}-apiKey` };
    case "npm":
      return { type: "npm", package: "" };
    case "gumroad":
      return { type: "gumroad", metric: "sales", token: `${base}-token` };
    case "lemonSqueezy":
      return { type: "lemonSqueezy", metric: "sales", apiKey: `${base}-apiKey` };
    case "productHunt":
      return { type: "productHunt", post: "", token: `${base}-token` };
    case "discord":
      return { type: "discord", invite: "", metric: "members" };
    case "formula":
      return { type: "formula", expr: "" };
    case "push":
      return { type: "push" };
  }
}

export function blankEntry(): Entry {
  return {
    id: 0,
    slug: "",
    name: "",
    icon: TYPES[0].icon,
    groupId: null,
    source: defaultSource("customUrl", `e${rid()}`),
    intervalS: 300,
    format: { style: "full", decimals: null, prefix: "", suffix: "" },
    goalValue: null,
    celebrate: true,
    sounds: { onChange: null, onGoal: "sparkle" },
    sort: 0,
  };
}

function secretNames(s: SourceCfg): string[] {
  switch (s.type) {
    case "customUrl":
      return s.headers.filter((h) => h.value.kind === "secret").map((h) => h.value.value);
    case "youtube":
    case "stripe":
    case "plausible":
    case "lemonSqueezy":
      return [s.apiKey];
    case "github":
      return s.token ? [s.token] : [];
    case "shopify":
    case "gumroad":
    case "productHunt":
      return [s.token];
    case "formula":
    case "push":
    case "npm":
    case "discord":
      return [];
  }
}

function SecretInput({ name, saved, value, onChange, placeholder }: { name: string; saved: boolean; value: string; onChange: (n: string, v: string) => void; placeholder: string }) {
  return (
    <input
      type="password"
      autoComplete="off"
      spellCheck={false}
      value={value}
      placeholder={saved ? "Saved in your keyring. Type to replace" : placeholder}
      onChange={(e) => onChange(name, e.target.value)}
    />
  );
}

interface Props {
  initial: Entry;
  views: EntryView[];
  groups: Group[];
  keyring: boolean;
  onClose: () => void;
  onSaved: (e: Entry) => void;
}

export function EntryEditor({ initial, views, groups, keyring, onClose, onSaved }: Props) {
  const [d, setD] = useState<Entry>(initial);
  const [base] = useState(() => `e${rid()}`);
  const [slugTouched, setSlugTouched] = useState(initial.id > 0);
  const [secrets, setSecrets] = useState<Record<string, string>>({});
  const [saved, setSaved] = useState<Record<string, boolean>>({});
  const [test, setTest] = useState<TestResult | null>(null);
  const [testErr, setTestErr] = useState("");
  const [testing, setTesting] = useState(false);
  const [err, setErr] = useState("");
  const [busy, setBusy] = useState(false);
  const [confirmDel, setConfirmDel] = useState(false);
  const [simValue, setSimValue] = useState<number | null>(null);
  const isNew = d.id === 0;
  const live = views.find((v) => v.entry.id === d.id);

  useEffect(() => {
    const names = secretNames(initial.source);
    if (names.length) api.hasSecrets(names).then((r) => setSaved(Object.fromEntries(names.map((n, i) => [n, r[i]]))));
  }, [initial]);

  const set = (patch: Partial<Entry>) => setD((x) => ({ ...x, ...patch }));
  const setSource = (s: SourceCfg) => {
    setD((x) => ({ ...x, source: s }));
    setTest(null);
    setTestErr("");
  };
  const setSecret = (n: string, v: string) => setSecrets((m) => ({ ...m, [n]: v }));
  const secretList = (): [string, string][] => Object.entries(secrets).filter(([n, v]) => v.trim() && secretNames(d.source).includes(n));
  const hasSecret = (n: string) => !!saved[n] || !!secrets[n]?.trim();
  /** Drops unused header rows; returns an error message when a secret header was left empty. */
  const cleaned = (): { source: SourceCfg; problem: string } => {
    const src = d.source;
    if (src.type !== "customUrl") return { source: src, problem: "" };
    const headers = src.headers.filter((h) => h.name.trim() || (h.value.kind === "secret" ? hasSecret(h.value.value) : h.value.value.trim()));
    const empty = headers.find((h) => h.value.kind === "secret" && !hasSecret(h.value.value));
    return {
      source: { ...src, headers },
      problem: empty ? `The ${empty.name.trim() || "header"} value is empty. Fill it in, or remove that header if the address needs no key.` : "",
    };
  };

  const chooseType = (t: SourceType) => {
    if (t === d.source.type) return;
    const def = TYPES.find((x) => x.id === t)!;
    const name = d.name && !Object.values(DEFAULT_NAMES).includes(d.name) ? d.name : DEFAULT_NAMES[t];
    setD((x) => ({
      ...x,
      source: defaultSource(t, base),
      textMode: TEXT_TYPES.includes(t) ? x.textMode : false,
      goalText: TEXT_TYPES.includes(t) ? x.goalText : null,
      icon: def.icon,
      name,
      slug: slugTouched ? x.slug : name ? slugify(name) : "",
      format: t === "stripe" ? { ...x.format, prefix: "€", decimals: 2 } : { ...x.format, prefix: "", decimals: null },
    }));
    setTest(null);
    setTestErr("");
  };

  const runTest = async () => {
    setTesting(true);
    setTest(null);
    setTestErr("");
    try {
      const { source, problem } = cleaned();
      if (problem) throw new Error(problem);
      setTest(await api.testSource(source, secretList()));
    } catch (e) {
      setTestErr(errorText(e));
    } finally {
      setTesting(false);
    }
  };

  const save = async () => {
    setErr("");
    if (!d.name.trim()) return setErr("Give this number a name.");
    const { source, problem } = cleaned();
    if (problem) return setErr(problem);
    setBusy(true);
    try {
      const out = await api.saveEntry({ ...d, source, slug: d.slug || slugify(d.name) }, secretList());
      onSaved(out);
    } catch (e) {
      setErr(errorText(e));
    } finally {
      setBusy(false);
    }
  };

  const del = async () => {
    if (!confirmDel) return setConfirmDel(true);
    try {
      await api.deleteEntry(d.id);
      onClose();
    } catch (e) {
      setErr(errorText(e));
      setConfirmDel(false);
    }
  };

  const src = d.source;
  const showsField = (
    <Field label="Shows" wide hint={d.textMode ? "Text such as a status, a title or a version. The goal can be a text it changes to." : undefined}>
      <Segmented
        value={d.textMode ? "text" : "number"}
        onChange={(v) => set({ textMode: v === "text", goalValue: v === "text" ? null : d.goalValue, goalText: v === "text" ? d.goalText : null })}
        options={[
          { id: "number", label: "A number" },
          { id: "text", label: "Text" },
        ]}
      />
    </Field>
  );
  const otherSlugs = useMemo(() => views.filter((v) => v.entry.id !== d.id).map((v) => v.entry.slug), [views, d.id]);

  return (
    <div className="editor" role="dialog" aria-label={isNew ? "Add a number" : `Edit ${d.name}`}>
      <header className="editor-head" data-tauri-drag-region>
        <button type="button" className="back-btn" onClick={onClose} aria-label="Back to numbers">
          <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" strokeWidth={2.2} strokeLinecap="round" strokeLinejoin="round">
            <path d="M15 5l-7 7 7 7" />
          </svg>
          Back
        </button>
        <EntryIcon icon={d.icon} size={30} />
        <div>
          <h2>{isNew ? "Add a number" : d.name || "Untitled"}</h2>
          {live && (
            <span className="muted">
              Now: <b>{formatValue(live.value, live.entry.format)}</b> · {live.status}
              {live.error ? ` · ${live.error}` : ""}
            </span>
          )}
        </div>
      </header>

      <div className="editor-body">
        <section>
          <h3>1 · Where the number comes from</h3>
          <div className="types">
            {TYPES.map((t) => (
              <button type="button" key={t.id} className={"type" + (src.type === t.id ? " on" : "")} onClick={() => chooseType(t.id)}>
                <EntryIcon icon={t.icon} size={26} />
                <b>{t.label}</b>
                <small>{t.hint}</small>
              </button>
            ))}
          </div>

          {!keyring && !KEYLESS_TYPES.includes(src.type) && (
            <Banner kind="error">Your system keyring is not available, so API keys cannot be stored safely. Unlock or install a keyring, then reopen Cownter.</Banner>
          )}

          {src.type === "customUrl" && (
            <div className="grid2">
              <Field label="URL" wide>
                <div className="row">
                  <select value={src.method} onChange={(e) => setSource({ ...src, method: e.target.value })} className="method">
                    <option>GET</option>
                    <option>POST</option>
                  </select>
                  <input value={src.url} placeholder="https://api.example.com/stats" onChange={(e) => setSource({ ...src, url: e.target.value })} />
                </div>
              </Field>
              <Field label="Headers" wide hint="Values marked secret are stored in your OS keyring, never in Cownter's files.">
                <div className="headers">
                  {src.headers.map((h, i) => {
                    const update = (nh: Header) => setSource({ ...src, headers: src.headers.map((x, j) => (j === i ? nh : x)) });
                    const isSecret = h.value.kind === "secret";
                    return (
                      <div className="row" key={i}>
                        <input className="hname" value={h.name} placeholder="Authorization" onChange={(e) => update({ ...h, name: e.target.value })} />
                        {isSecret ? (
                          <SecretInput name={h.value.value} saved={!!saved[h.value.value]} value={secrets[h.value.value] ?? ""} onChange={setSecret} placeholder="Bearer …" />
                        ) : (
                          <input value={h.value.value} placeholder="value" onChange={(e) => update({ ...h, value: { kind: "text", value: e.target.value } })} />
                        )}
                        <label className="check">
                          <input
                            type="checkbox"
                            checked={isSecret}
                            onChange={(e) => update({ ...h, value: e.target.checked ? { kind: "secret", value: `${base}-h${rid()}` } : { kind: "text", value: "" } })}
                          />
                          secret
                        </label>
                        <button type="button" className="icon-btn" aria-label="Remove header" onClick={() => setSource({ ...src, headers: src.headers.filter((_, j) => j !== i) })}>
                          ✕
                        </button>
                      </div>
                    );
                  })}
                  <button type="button" className="btn ghost" onClick={() => setSource({ ...src, headers: [...src.headers, { name: "", value: { kind: "text", value: "" } }] })}>
                    + Header
                  </button>
                </div>
              </Field>
              {src.method === "POST" && (
                <Field label="Body" wide>
                  <textarea rows={3} value={src.body ?? ""} onChange={(e) => setSource({ ...src, body: e.target.value || null })} />
                </Field>
              )}
              {showsField}
              <Field label={d.textMode ? "Which text" : "Which number"} wide>
                <Segmented
                  value={src.pick.kind}
                  onChange={(k) => setSource({ ...src, pick: k === "firstNumber" ? { kind: "firstNumber" } : ({ kind: k, value: "" } as Pick) })}
                  options={[
                    { id: "jsonPath", label: "Pick from JSON" },
                    { id: "firstNumber", label: d.textMode ? "First line" : "First number" },
                    { id: "regex", label: "Pattern" },
                  ]}
                />
                {src.pick.kind === "jsonPath" && (
                  <input value={src.pick.value} placeholder="Click a number after testing, or type data.stats.count" onChange={(e) => setSource({ ...src, pick: { kind: "jsonPath", value: e.target.value } })} />
                )}
                {src.pick.kind === "regex" && (
                  <input value={src.pick.value} placeholder="orders: (\d+)" onChange={(e) => setSource({ ...src, pick: { kind: "regex", value: e.target.value } })} />
                )}
              </Field>
            </div>
          )}

          {src.type === "youtube" && (
            <div className="grid2">
              <Field label="Channel ID" hint="Starts with UC… Find it in YouTube Studio → Settings → Channel → Advanced.">
                <input value={src.channelId} placeholder="UCxxxxxxxxxxxxxxxxxxxxxx" onChange={(e) => setSource({ ...src, channelId: e.target.value })} />
              </Field>
              <Field label="Metric">
                <select value={src.metric} onChange={(e) => setSource({ ...src, metric: e.target.value as typeof src.metric })}>
                  <option value="subscribers">Subscribers</option>
                  <option value="views">Total views</option>
                  <option value="videos">Videos</option>
                </select>
              </Field>
              <Field label="YouTube Data API key" wide hint="Create one in Google Cloud Console → APIs & Services → Credentials.">
                <SecretInput name={src.apiKey} saved={!!saved[src.apiKey]} value={secrets[src.apiKey] ?? ""} onChange={setSecret} placeholder="AIza…" />
              </Field>
            </div>
          )}

          {src.type === "github" && (
            <div className="grid2">
              <Field label="Repository or user" hint="owner/repo for repository metrics, a user name for followers.">
                <input value={src.target} placeholder="tauri-apps/tauri" onChange={(e) => setSource({ ...src, target: e.target.value })} />
              </Field>
              <Field label="Metric">
                <select value={src.metric} onChange={(e) => setSource({ ...src, metric: e.target.value as typeof src.metric })}>
                  <option value="stars">Stars</option>
                  <option value="forks">Forks</option>
                  <option value="openIssues">Open issues</option>
                  <option value="watchers">Watchers</option>
                  <option value="followers">Followers (user)</option>
                  <option value="publicRepos">Public repos (user)</option>
                </select>
              </Field>
              <Field label="Token (optional)" wide hint="Raises the rate limit from 60 to 5,000 requests per hour.">
                <div className="row">
                  <Toggle checked={!!src.token} onChange={(on) => setSource({ ...src, token: on ? `${base}-token` : null })} label="Use a token" />
                  {src.token && <SecretInput name={src.token} saved={!!saved[src.token]} value={secrets[src.token] ?? ""} onChange={setSecret} placeholder="github_pat_…" />}
                </div>
              </Field>
            </div>
          )}

          {src.type === "shopify" && (
            <div className="grid2">
              <Field label="Shop domain">
                <input value={src.shop} placeholder="my-store.myshopify.com" onChange={(e) => setSource({ ...src, shop: e.target.value })} />
              </Field>
              <Field label="Admin API access token" hint="Custom app with the read_orders scope.">
                <SecretInput name={src.token} saved={!!saved[src.token]} value={secrets[src.token] ?? ""} onChange={setSecret} placeholder="shpat_…" />
              </Field>
            </div>
          )}

          {src.type === "stripe" && (
            <div className="grid2">
              <Field label="Restricted API key" hint="Read-only access to Charges is enough.">
                <SecretInput name={src.apiKey} saved={!!saved[src.apiKey]} value={secrets[src.apiKey] ?? ""} onChange={setSecret} placeholder="rk_live_…" />
              </Field>
              <Field label="Currency">
                <input value={src.currency} maxLength={3} onChange={(e) => setSource({ ...src, currency: e.target.value.toUpperCase() })} />
              </Field>
            </div>
          )}

          {src.type === "plausible" && (
            <div className="grid2">
              <Field label="Site" hint="The domain as it is added in Plausible.">
                <input value={src.siteId} placeholder="example.com" onChange={(e) => setSource({ ...src, siteId: e.target.value })} />
              </Field>
              <Field label="Metric">
                <select value={src.metric} onChange={(e) => setSource({ ...src, metric: e.target.value as typeof src.metric })}>
                  <option value="visitorsToday">Visitors today</option>
                  <option value="realtime">Visitors right now</option>
                </select>
              </Field>
              <Field label="Stats API key" wide hint="Plausible → Account settings → API keys.">
                <SecretInput name={src.apiKey} saved={!!saved[src.apiKey]} value={secrets[src.apiKey] ?? ""} onChange={setSecret} placeholder="API key" />
              </Field>
            </div>
          )}

          {src.type === "npm" && (
            <div className="grid2">
              <Field label="Package" wide hint="Downloads over the last 7 days. No key needed.">
                <input value={src.package} placeholder="react or @scope/name" onChange={(e) => setSource({ ...src, package: e.target.value })} />
              </Field>
            </div>
          )}

          {src.type === "gumroad" && (
            <div className="grid2">
              <Field label="Metric" hint="Summed over all your products.">
                <select value={src.metric} onChange={(e) => setSource({ ...src, metric: e.target.value as typeof src.metric })}>
                  <option value="sales">Sales</option>
                  <option value="revenue">Revenue (USD)</option>
                </select>
              </Field>
              <Field label="Access token" hint="Settings → Advanced → Applications: create an application and generate a token.">
                <SecretInput name={src.token} saved={!!saved[src.token]} value={secrets[src.token] ?? ""} onChange={setSecret} placeholder="Access token" />
              </Field>
            </div>
          )}

          {src.type === "lemonSqueezy" && (
            <div className="grid2">
              <Field label="Metric" hint="Summed over all your stores, in the store currency.">
                <select value={src.metric} onChange={(e) => setSource({ ...src, metric: e.target.value as typeof src.metric })}>
                  <option value="sales">Sales (all time)</option>
                  <option value="revenue">Revenue (all time)</option>
                  <option value="revenue30d">Revenue (last 30 days)</option>
                </select>
              </Field>
              <Field label="API key" hint="Settings → API.">
                <SecretInput name={src.apiKey} saved={!!saved[src.apiKey]} value={secrets[src.apiKey] ?? ""} onChange={setSecret} placeholder="API key" />
              </Field>
            </div>
          )}

          {src.type === "productHunt" && (
            <div className="grid2">
              <Field label="Post" hint="The post link or the slug after /posts/.">
                <input value={src.post} placeholder="https://www.producthunt.com/posts/…" onChange={(e) => setSource({ ...src, post: e.target.value })} />
              </Field>
              <Field label="Developer token" hint="API dashboard → add an application → Developer token.">
                <SecretInput name={src.token} saved={!!saved[src.token]} value={secrets[src.token] ?? ""} onChange={setSecret} placeholder="Developer token" />
              </Field>
            </div>
          )}

          {src.type === "discord" && (
            <div className="grid2">
              <Field label="Invite" hint="An invite link that never expires. No key needed.">
                <input value={src.invite} placeholder="https://discord.gg/…" onChange={(e) => setSource({ ...src, invite: e.target.value })} />
              </Field>
              <Field label="Metric" hint="Discord gives approximate counts.">
                <select value={src.metric} onChange={(e) => setSource({ ...src, metric: e.target.value as typeof src.metric })}>
                  <option value="members">Members</option>
                  <option value="online">Online now</option>
                </select>
              </Field>
            </div>
          )}

          {src.type === "formula" && (
            <Field label="Formula" wide hint="Use + − × ÷ and parentheses. Click a name to insert it.">
              <input className="mono" value={src.expr} placeholder="revenue / orders" onChange={(e) => setSource({ ...src, expr: e.target.value })} />
              <div className="slugs">
                {otherSlugs.length === 0 && <span className="muted">Add other numbers first.</span>}
                {otherSlugs.map((s) => (
                  <button type="button" key={s} className="slug-chip" onClick={() => setSource({ ...src, expr: (src.expr ? src.expr.trimEnd() + " " : "") + s })}>
                    {s}
                  </button>
                ))}
              </div>
            </Field>
          )}

          {src.type === "push" && (
            <div className="grid2">
              {showsField}
              <Field label="How to send values" wide hint="The address and token are on the General page.">
                <code className="mono push-example">cownter set "{d.name.trim() || d.slug || "Name"}" {d.textMode ? "green" : "42"}</code>
              </Field>
            </div>
          )}

          {src.type !== "formula" && src.type !== "push" && (
            <div className="test-row">
              <button type="button" className="btn" onClick={runTest} disabled={testing}>
                {testing ? "Testing…" : "Test"}
              </button>
              {test && src.type !== "customUrl" && test.value !== null && (
                <span className="ok-text">
                  Got <b>{formatValue(test.value, d.format)}</b> in {test.elapsedMs} ms
                </span>
              )}
              {testErr && <span className="field-error">{testErr}</span>}
            </div>
          )}
          {test && src.type === "customUrl" && <SmartPicker result={test} pick={src.pick} text={!!d.textMode} onPick={(p) => setSource({ ...src, pick: p })} />}
        </section>

        <section>
          <h3>2 · Name and look</h3>
          <div className="grid2">
            <Field label="Name">
              <input
                value={d.name}
                placeholder="Orders today"
                onChange={(e) => set({ name: e.target.value, slug: slugTouched ? d.slug : slugify(e.target.value) })}
              />
            </Field>
            <Field label="Short name" hint="Used in formulas.">
              <input
                className="mono"
                value={d.slug}
                onChange={(e) => {
                  setSlugTouched(true);
                  set({ slug: e.target.value.toLowerCase().replace(/[^a-z0-9_]/g, "_").slice(0, 40) });
                }}
              />
            </Field>
            <Field label="Icon" wide>
              <IconPicker icon={d.icon} onChange={(icon) => set({ icon })} siteUrl={src.type === "customUrl" ? src.url : undefined} />
            </Field>
            <Field label="Group">
              <select value={d.groupId ?? ""} onChange={(e) => set({ groupId: e.target.value ? Number(e.target.value) : null })}>
                <option value="">No group</option>
                {groups.map((g) => (
                  <option key={g.id} value={g.id}>
                    {g.name}
                  </option>
                ))}
              </select>
            </Field>
            {src.type !== "formula" && src.type !== "push" && (
              <Field label="Check every">
                <select value={d.intervalS} onChange={(e) => set({ intervalS: Number(e.target.value) })}>
                  {(INTERVALS.includes(d.intervalS) ? INTERVALS : [...INTERVALS, d.intervalS].sort((a, b) => a - b)).map((s) => (
                    <option key={s} value={s}>
                      {intervalLabel(s)}
                    </option>
                  ))}
                </select>
              </Field>
            )}
            <Field label="Number style">
              <Segmented
                value={d.format.style}
                onChange={(style) => set({ format: { ...d.format, style } })}
                options={[
                  { id: "full", label: "48,213" },
                  { id: "compact", label: "48.2K" },
                ]}
              />
            </Field>
            <Field label="Decimals">
              <select value={d.format.decimals ?? ""} onChange={(e) => set({ format: { ...d.format, decimals: e.target.value === "" ? null : Number(e.target.value) } })}>
                <option value="">Auto</option>
                {[0, 1, 2, 3].map((n) => (
                  <option key={n} value={n}>
                    {n}
                  </option>
                ))}
              </select>
            </Field>
            <Field label="Prefix">
              <input value={d.format.prefix} maxLength={4} placeholder="€" onChange={(e) => set({ format: { ...d.format, prefix: e.target.value } })} />
            </Field>
            <Field label="Suffix">
              <input value={d.format.suffix} maxLength={4} placeholder="%" onChange={(e) => set({ format: { ...d.format, suffix: e.target.value } })} />
            </Field>
          </div>
        </section>

        <section>
          <h3>3 · Goal</h3>
          <p className="sec-hint">Optional. Shows progress toward a target and celebrates when it is reached.</p>
          <div className="sec-rows">
            <div className="sec-row">
              <span className="sec-label">{d.textMode ? "When the text becomes" : "Target"}</span>
              {d.textMode ? (
                <input type="text" value={d.goalText ?? ""} placeholder="e.g. Delivered" maxLength={200} onChange={(e) => set({ goalText: e.target.value || null })} />
              ) : (
                <NumberInput value={d.goalValue} placeholder="e.g. 10000" onChange={(goalValue) => set({ goalValue })} />
              )}
            </div>
            {!d.textMode && live && live.value !== null && d.goalValue !== null && live.entry.goalValue === d.goalValue && (
              <div className="sec-row">
                <span className="sec-label">Forecast</span>
                <span className="muted" title="Based on the trend of the last 30 days">
                  {live.value >= d.goalValue ? "Reached" : goalLeft(live) + (live.goalEta ? "" : " · no date yet, it needs a rising trend")}
                </span>
              </div>
            )}
            <div className="sec-row">
              <span className="sec-label">Confetti on goals and new records</span>
              <Toggle checked={d.celebrate} onChange={(celebrate) => set({ celebrate })} />
            </div>
          </div>
        </section>

        <section>
          <h3>4 · Sounds</h3>
          <p className="sec-hint">A short sound when something happens. Volume is on the General page.</p>
          <div className="sec-rows">
            {(
              [
                ["onChange", "When the number changes"],
                ["onGoal", "When it reaches the goal or a new record"],
              ] as const
            ).map(([key, label]) => (
              <div className="sec-row" key={key}>
                <span className="sec-label">{label}</span>
                <div className="row">
                  <select value={d.sounds[key] ?? ""} onChange={(e) => set({ sounds: { ...d.sounds, [key]: e.target.value || null } })}>
                    <option value="">No sound</option>
                    {SOUNDS.map((x) => (
                      <option key={x.id} value={x.id}>
                        {x.label}
                      </option>
                    ))}
                  </select>
                  <button type="button" className="icon-btn" aria-label="Play" disabled={!d.sounds[key]} onClick={() => playSound(d.sounds[key], 0.7)}>
                    ▶
                  </button>
                </div>
              </div>
            ))}
          </div>
        </section>

        <section>
          <h3>5 · Alerts</h3>
          <p className="sec-hint">Rules that react to the number: a desktop notification, an effect such as +1 on the widget, or both.</p>
          {!isNew ? <AlertsEditor entryId={d.id} view={live} /> : <p className="muted">Save this number first, then add alerts here.</p>}
        </section>

        {!isNew && (
          <section>
            <h3>6 · Try it</h3>
            <p className="sec-hint">Records a fake reading so you can preview animations, sounds, alerts and confetti.</p>
            <div className="row">
              <NumberInput value={simValue} placeholder={live?.value !== null && live?.value !== undefined ? String(live.value + 1) : "value"} onChange={setSimValue} />
              <button
                type="button"
                className="btn ghost"
                onClick={() => {
                  const v = simValue ?? (live?.value ?? 0) + 1;
                  api.simulateChange(d.id, v).catch((e) => setErr(errorText(e)));
                }}
              >
                Simulate a change
              </button>
            </div>
          </section>
        )}
      </div>

      <footer className="editor-foot">
        {err && <span className="field-error">{err}</span>}
        {!isNew && (
          <button type="button" className={"btn danger" + (confirmDel ? " armed" : "")} onClick={del} onBlur={() => setConfirmDel(false)}>
            {confirmDel ? "Click again to delete" : "Delete"}
          </button>
        )}
        <span className="spacer" />
        <button type="button" className="btn ghost" onClick={onClose}>
          Cancel
        </button>
        <button type="button" className="btn primary" onClick={save} disabled={busy}>
          {busy ? "Saving…" : isNew ? "Add number" : "Save"}
        </button>
      </footer>
    </div>
  );
}
