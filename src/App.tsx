import { useEffect, useState } from "react";
import { Disclaimer } from "@/components/Disclaimer";
import { HardwareBar } from "@/components/HardwareBar";
import { Logo } from "@/components/Logo";
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
  const [reread, setReread] = useState(false);
  const [update, setUpdate] = useState<UpdateInfo | null>(null);

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
    void api.checkUpdate().then(setUpdate).catch(() => undefined);
  }, [setTheme]);

  useEffect(() => {
    if (!prefs?.disclaimerAccepted) {
      return;
    }
    void refresh();
    const id = window.setInterval(() => {
      void api.liveMetrics().then(setMetrics).catch(() => undefined);
    }, 2000);
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
    return <div className="grid h-dvh place-items-center text-sm text-muted-foreground">{error || "Loading"}</div>;
  }
  if (!prefs.disclaimerAccepted) {
    return <Disclaimer onAccept={() => void accept()} onQuit={() => void api.closeApp()} />;
  }
  if (!report) {
    return <div className="grid h-dvh place-items-center text-sm text-muted-foreground">{error || copy.detectFailed}</div>;
  }

  return (
    <div className="flex h-dvh flex-col">
      <header className="space-y-3 border-b px-5 py-3">
        <div className="flex flex-wrap items-start justify-between gap-3">
          <div className="flex items-start gap-3">
            <Logo />
            <div>
              <h1 className="text-lg font-semibold">{copy.appName}</h1>
              <p className="text-sm text-muted-foreground">{copy.tagline}</p>
              <p className="text-xs text-muted-foreground">{copy.unofficial}</p>
            </div>
          </div>
          <div className="flex flex-wrap items-center gap-3">
            <div className="flex items-center gap-2">
              <Switch
                checked={prefs.telemetry}
                onCheckedChange={(on) => void persist({ ...prefs, telemetry: on })}
              />
              <Label className="text-xs font-normal">{copy.telemetry}</Label>
            </div>
            <Button
              size="sm"
              variant="outline"
              onClick={() => {
                const next = theme === "dark" ? "light" : "dark";
                setTheme(next);
                void persist({ ...prefs, theme: next });
              }}
            >
              {theme === "dark" ? copy.themeLight : copy.themeDark}
            </Button>
            <Button size="sm" variant="ghost" onClick={() => setReread(true)}>
              {copy.rereadDisclaimer}
            </Button>
          </div>
        </div>
        {!elevated ? (
          <Alert>
            {copy.adminNeeded}{" "}
            <Button size="sm" className="ml-2" onClick={() => void api.relaunchElevated()}>
              {copy.relaunchAdmin}
            </Button>
          </Alert>
        ) : null}
        {error ? <Alert>{error}</Alert> : null}
        {update && !update.available ? <p className="text-xs text-muted-foreground">{copy.updateNone}</p> : null}
        <HardwareBar hardware={report.hardware} metrics={metrics} />
      </header>
      <nav className="flex flex-wrap gap-1 border-b px-5 py-2">
        {(Object.keys(copy.tabs) as TabId[]).map((id) => (
          <Button key={id} size="sm" variant={tab === id ? "default" : "ghost"} onClick={() => setTab(id)}>
            {copy.tabs[id]}
          </Button>
        ))}
      </nav>
      <main data-ui-scroll-container className="min-h-0 flex-1 px-5 py-4">
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
        {tab === "tune" ? <TunePage /> : null}
        {tab === "fix" ? <FixPage /> : null}
        {tab === "reference" ? <ReferencePage /> : null}
        {tab === "log" ? <LogPage /> : null}
      </main>
      <Dialog open={reread} onOpenChange={setReread}>
        <DialogContent>
          <DialogTitle>{copy.disclaimerTitle}</DialogTitle>
          <DialogDescription asChild>
            <div className="ui-selectable max-h-80 space-y-3 overflow-y-auto text-sm">
              {copy.disclaimerBody.map((p) => (
                <p key={p}>{p}</p>
              ))}
            </div>
          </DialogDescription>
          <Button className="mt-4" variant="outline" onClick={() => setReread(false)}>
            {copy.quit}
          </Button>
        </DialogContent>
      </Dialog>
    </div>
  );
}
