import { lazy, Suspense, useEffect, useState } from "react";
import { Disclaimer } from "@/components/Disclaimer";
import { Loading } from "@/components/Loading";
import { Alert } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogTitle } from "@/components/ui/dialog";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { useTheme } from "@/components/theme-provider";
import * as api from "@/lib/api";
import { copy } from "@/lib/copy";
import { FixPage } from "@/pages/FixPage";
import { LogPage } from "@/pages/LogPage";
import { OptimizePage } from "@/pages/OptimizePage";
import { ReferencePage } from "@/pages/ReferencePage";
import { TunePage } from "@/pages/TunePage";
import type { DetectReport, ItemResult, LiveMetrics, Prefs, Preset, RestoreItem, TabId, UpdateInfo } from "@/lib/types";

const HardwareBar = lazy(async () => {
  const mod = await import("@/components/HardwareBar");
  return { default: mod.HardwareBar };
});

export function App() {
  const { theme, setTheme } = useTheme();
  const [prefs, setPrefs] = useState<Prefs | null>(null);
  const [report, setReport] = useState<DetectReport | null>(null);
  const [presets, setPresets] = useState<Preset[]>([]);
  const [restoreItems, setRestoreItems] = useState<RestoreItem[]>([]);
  const [metrics, setMetrics] = useState<LiveMetrics | null>(null);
  const [tab, setTab] = useState<TabId>("optimize");
  const [gamePath, setGamePath] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [lastApply, setLastApply] = useState<ItemResult[] | null>(null);
  const [elevated, setElevated] = useState(true);
  const [settings, setSettings] = useState(false);
  const [update, setUpdate] = useState<UpdateInfo | null>(null);
  const [version, setVersion] = useState("");

  useEffect(() => {
    void api
      .getPrefs()
      .then((next) => {
        setPrefs(next);
        if (next.theme === "light" || next.theme === "dark") {
          setTheme(next.theme);
        }
      })
      .catch((e: Error) => setError(e.message));
    void api.isElevated().then(setElevated);
    void api.appVersion().then(setVersion).catch(() => undefined);
  }, [setTheme]);

  useEffect(() => {
    if (!prefs?.disclaimerAccepted) {
      return;
    }
    void refresh();
    void api.checkUpdate().then(setUpdate).catch(() => undefined);
    void api.liveMetrics().then(setMetrics).catch(() => undefined);
    const id = window.setInterval(() => {
      void api.liveMetrics().then(setMetrics).catch(() => undefined);
    }, 1000);
    return () => window.clearInterval(id);
  }, [prefs?.disclaimerAccepted]);

  async function refresh(path?: string) {
    setError("");
    try {
      const next = await api.detect(path ?? (gamePath || null));
      setReport(next);
      setGamePath((next.gamePath ?? path) ?? "");
      setPresets(await api.listPresets());
      setRestoreItems(await api.listRestore());
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }

  async function persist(next: Prefs) {
    setPrefs(await api.setPrefs(next));
  }

  async function accept() {
    if (!prefs) {
      return;
    }
    await persist({ ...prefs, disclaimerAccepted: true });
  }

  async function onApply(items: string[], risky: boolean, spoof: string | null) {
    setBusy(true);
    setError("");
    try {
      const reportApply = await api.applyItems({
        items,
        gamePath: gamePath || null,
        gpuSpoofModel: spoof,
        risky,
      });
      setLastApply(reportApply.results);
      await refresh(gamePath);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }

  async function onRestore(items: string[] | null) {
    setBusy(true);
    setError("");
    try {
      const restored = await api.restoreItems(items);
      setLastApply(restored.results);
      await refresh(gamePath);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }

  if (!prefs) {
    if (error) {
      return (
        <div className="grid h-dvh place-items-center gap-2 text-xs text-muted-foreground">
          <p>{error}</p>
        </div>
      );
    }
    return <Loading label={copy.loading} />;
  }
  if (!prefs.disclaimerAccepted) {
    return <Disclaimer onAccept={() => void accept()} onQuit={() => void api.closeApp()} />;
  }
  if (!report) {
    if (error) {
      return (
        <div className="grid h-dvh place-items-center gap-2">
          <p className="text-xs text-muted-foreground">{error || copy.detectFailed}</p>
          <Button onClick={() => void refresh()}>{copy.retry}</Button>
        </div>
      );
    }
    return <Loading label={copy.loading} />;
  }

  return (
    <div className="flex h-dvh min-h-0 flex-col overflow-hidden">
      <header className="grid h-11 shrink-0 grid-cols-[minmax(0,1fr)_auto] items-center gap-2 overflow-hidden border-b px-3">
        <nav className="relative z-10 flex min-w-0 items-center gap-0.5 overflow-x-auto bg-background">
          {(Object.keys(copy.tabs) as TabId[]).map((id) => (
            <Button key={id} variant={tab === id ? "default" : "ghost"} onClick={() => setTab(id)}>
              {copy.tabs[id]}
            </Button>
          ))}
        </nav>
        <div className="relative z-0 flex shrink-0 items-center gap-2 overflow-hidden border-l pl-3">
          <Suspense fallback={null}>
            <HardwareBar hardware={report.hardware} metrics={metrics} />
          </Suspense>
          <Button
            variant="ghost"
            onClick={() => {
              setSettings(true);
              void api.checkUpdate().then(setUpdate).catch(() => undefined);
            }}
          >
            {copy.settings}
          </Button>
        </div>
      </header>
      {!elevated ? (
        <Alert className="mx-3 mt-2 shrink-0 py-1.5 text-xs">
          {copy.adminNeeded}{" "}
          <Button className="ml-1" onClick={() => void api.relaunchElevated()}>
            {copy.relaunchAdmin}
          </Button>
        </Alert>
      ) : null}
      {error ? <Alert className="mx-3 mt-2 shrink-0 py-1.5 text-xs">{error}</Alert> : null}
      <main className="min-h-0 flex-1 overflow-hidden px-3 py-2">
        {tab === "optimize" ? (
          <OptimizePage
            report={report}
            presets={presets}
            gamePath={gamePath}
            busy={busy}
            lastApply={lastApply}
            restoreItems={restoreItems}
            onGamePath={setGamePath}
            onFind={() => void api.findGame().then((p) => p && void refresh(p))}
            onBrowse={() => void api.pickGame().then((p) => p && void refresh(p))}
            onApply={onApply}
            onRestore={(items) => void onRestore(items)}
            onSavePreset={(name, items) =>
              void api.savePreset(name, items).then(() => api.listPresets().then(setPresets))
            }
            onDeletePreset={(id) => void api.deletePreset(id).then(() => api.listPresets().then(setPresets))}
          />
        ) : null}
        {tab === "tune" ? <TunePage gamePath={gamePath} /> : null}
        {tab === "fix" ? <FixPage gpuGuide={report.gpuGuide} /> : null}
        {tab === "reference" ? <ReferencePage /> : null}
        {tab === "log" ? <LogPage /> : null}
      </main>
      <Dialog open={settings} onOpenChange={setSettings}>
        <DialogContent>
          <div className="grid gap-4 sm:grid-cols-2 sm:items-start">
            <div>
              <DialogTitle>{copy.settings}</DialogTitle>
              <DialogDescription className="mt-1">{copy.unofficial}</DialogDescription>
              <p className="mt-2 text-sm text-muted-foreground">{copy.tagline}</p>
              <p className="mt-2 text-xs text-muted-foreground">
                {copy.version} {update?.current ?? version}
                {update?.latest ? ` · latest ${update.latest}` : ""}
              </p>
              <p className="mt-1 text-xs text-muted-foreground">
                {update?.available ? update.notes || copy.updateReady : copy.updateNone}
              </p>
              <div className="mt-3 flex items-center gap-2">
                <Switch
                  checked={prefs.telemetry}
                  onCheckedChange={(on) => void persist({ ...prefs, telemetry: on })}
                />
                <Label className="text-xs font-normal">{copy.telemetry}</Label>
              </div>
              <div className="mt-3 flex flex-wrap gap-2">
                {update?.setupUrl ? (
                  <Button variant="outline" asChild>
                    <a href={update.setupUrl} target="_blank" rel="noreferrer">
                      {copy.updateOpen}
                    </a>
                  </Button>
                ) : null}
                {update?.available ? (
                  <Button
                    variant="outline"
                    onClick={() => {
                      void api
                        .downloadUpdate()
                        .then(() => api.checkUpdate().then(setUpdate))
                        .catch((e: Error) => setError(e.message));
                    }}
                  >
                    {copy.updateDownload}
                  </Button>
                ) : null}
                <Button
                  variant="outline"
                  onClick={() => {
                    const next = theme === "dark" ? "light" : "dark";
                    setTheme(next);
                    void persist({ ...prefs, theme: next });
                  }}
                >
                  {theme === "dark" ? copy.themeLight : copy.themeDark}
                </Button>
                <Button variant="ghost" onClick={() => setSettings(false)}>
                  {copy.quit}
                </Button>
              </div>
            </div>
            <div className="ui-selectable max-h-52 space-y-2 overflow-y-auto border p-3 text-xs text-muted-foreground">
              {copy.disclaimerBody.map((p) => (
                <p key={p}>{p}</p>
              ))}
            </div>
          </div>
        </DialogContent>
      </Dialog>
    </div>
  );
}
