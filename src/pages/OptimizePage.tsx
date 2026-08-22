import { useMemo, useState } from "react";
import { ItemRow } from "@/components/ItemRow";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Checkbox } from "@/components/ui/checkbox";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { copy } from "@/lib/copy";
import type { DetectReport, ItemResult, ItemView, RestoreItem, Preset } from "@/lib/types";

type Props = {
  report: DetectReport;
  presets: Preset[];
  gamePath: string;
  busy: boolean;
  lastApply: ItemResult[] | null;
  restoreItems: RestoreItem[];
  onGamePath: (path: string) => void;
  onFind: () => void;
  onBrowse: () => void;
  onApply: (items: string[], risky: boolean, spoof: string | null) => void;
  onRestore: (items: string[] | null) => void;
  onSavePreset: (name: string, items: string[]) => void;
  onDeletePreset: (id: string) => void;
};

function groupItems(items: ItemView[]) {
  const windows: ItemView[] = [];
  const game: ItemView[] = [];
  const checks: ItemView[] = [];
  for (const item of items) {
    if (item.kind === "check" || item.kind === "cache") {
      checks.push(item);
    } else if (item.requiresGame) {
      game.push(item);
    } else {
      windows.push(item);
    }
  }
  return [
    { id: "windows", title: copy.secWindows, items: windows },
    { id: "game", title: copy.secGame, items: game },
    { id: "checks", title: copy.secChecks, items: checks },
  ].filter((g) => g.items.length > 0);
}

export function OptimizePage({
  report,
  presets,
  gamePath,
  busy,
  lastApply,
  restoreItems,
  onGamePath,
  onFind,
  onBrowse,
  onApply,
  onRestore,
  onSavePreset,
  onDeletePreset,
}: Props) {
  const [selected, setSelected] = useState<Set<string>>(
    () => new Set(report.items.filter((i) => i.default && i.bulkSelect).map((i) => i.id)),
  );
  const [presetId, setPresetId] = useState("balanced");
  const [presetName, setPresetName] = useState("");
  const [more, setMore] = useState(false);
  const [guide, setGuide] = useState(false);
  const [confirmRisky, setConfirmRisky] = useState(false);
  const [restoreOpen, setRestoreOpen] = useState(false);
  const [restorePick, setRestorePick] = useState<Set<string>>(new Set());
  const [spoof, setSpoof] = useState(report.recommendedSpoof ?? report.spoofModels[0] ?? "");

  const selectedList = useMemo(() => [...selected], [selected]);
  const hasSpoof = selected.has("gpu-name-spoof");
  const preset = presets.find((p) => p.id === presetId);
  const groups = useMemo(() => groupItems(report.items), [report.items]);
  const selective = restoreItems.filter((i) => i.selective);

  const wrote = lastApply?.filter((r) => r.ok && r.changed).length ?? 0;
  const failed = lastApply?.filter((r) => !r.ok).length ?? 0;
  const skipped = lastApply?.filter((r) => r.skipped).length ?? 0;
  const rebootN = lastApply?.filter((r) => r.reboot).length ?? 0;

  function toggle(id: string, next: boolean) {
    setSelected((cur) => {
      const copySet = new Set(cur);
      if (next) {
        copySet.add(id);
      } else {
        copySet.delete(id);
      }
      return copySet;
    });
  }

  function loadPreset(id: string) {
    setPresetId(id);
    const next = presets.find((p) => p.id === id);
    if (!next) {
      return;
    }
    setSelected(new Set(next.items));
  }

  function requestApply() {
    if (hasSpoof) {
      setConfirmRisky(true);
      return;
    }
    onApply(selectedList, false, null);
  }

  return (
    <div className="flex h-full min-h-0 flex-col">
      <div className="flex shrink-0 flex-wrap items-center gap-1.5">
        <Input
          className="min-w-[160px] flex-1"
          placeholder={copy.gamePath}
          value={gamePath}
          onChange={(e) => onGamePath(e.target.value)}
        />
        <Button variant="outline" onClick={onFind} disabled={busy}>
          {copy.find}
        </Button>
        <Button variant="outline" onClick={onBrowse} disabled={busy}>
          {copy.browse}
        </Button>
        <Select value={presetId} onValueChange={loadPreset}>
          <SelectTrigger className="w-[150px]">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {presets.map((p) => (
              <SelectItem key={p.id} value={p.id}>
                {p.name}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <Button variant="ghost" onClick={() => setMore((v) => !v)}>
          {more ? copy.hide : copy.more}
        </Button>
      </div>

      {more ? (
        <div className="mt-2 flex shrink-0 flex-wrap items-center gap-1.5">
          <Button
            variant="outline"
            onClick={() => setSelected(new Set(report.items.filter((i) => i.default && i.bulkSelect).map((i) => i.id)))}
          >
            {copy.selectDefaults}
          </Button>
          <Button variant="outline" onClick={() => setSelected(new Set())}>
            {copy.selectNone}
          </Button>
          <Input
            className="max-w-[140px]"
            placeholder="Preset name"
            value={presetName}
            onChange={(e) => setPresetName(e.target.value)}
          />
          <Button variant="outline" onClick={() => presetName && onSavePreset(presetName, selectedList)}>
            {copy.savePreset}
          </Button>
          <Button variant="outline" onClick={() => onDeletePreset(presetId)} disabled={preset?.builtin}>
            {copy.deletePreset}
          </Button>
          <Button
            variant="outline"
            disabled={busy || selective.length === 0}
            onClick={() => {
              setRestorePick(new Set(selective.map((i) => i.id)));
              setRestoreOpen(true);
            }}
          >
            {copy.restoreSelected}
          </Button>
        </div>
      ) : null}

      {hasSpoof && report.spoofModels.length > 0 ? (
        <div className="mt-2 max-w-xs shrink-0">
          <Select value={spoof} onValueChange={setSpoof}>
            <SelectTrigger>
              <SelectValue placeholder={copy.spoofAs} />
            </SelectTrigger>
            <SelectContent>
              {report.spoofModels.map((model) => (
                <SelectItem key={model} value={model}>
                  {model}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
      ) : null}

      <div data-ui-scroll-container className="mt-2 min-h-0 flex-1 overflow-y-auto">
        {groups.map((group) => (
          <section key={group.id} className="mb-3 last:mb-0">
            <h2 className="mb-0.5 text-[11px] font-medium uppercase tracking-wide text-muted-foreground">
              {group.title}
            </h2>
            <div className="rounded-none border px-2">
              {group.items.map((item) => (
                <ItemRow key={item.id} item={item} checked={selected.has(item.id)} onToggle={toggle} />
              ))}
            </div>
          </section>
        ))}

        {lastApply ? (
          <p className="mt-2 text-[11px] text-muted-foreground">
            {wrote} {copy.wrote}
            {failed ? ` · ${failed} ${copy.failed}` : ""}
            {skipped ? ` · ${skipped} ${copy.skipped}` : ""}
            {rebootN ? ` · ${rebootN} ${copy.reboot}` : ""}
          </p>
        ) : null}

        {report.gpuGuide.length ? (
          <div className="mt-1">
            <button type="button" className="text-[11px] text-muted-foreground underline-offset-2 hover:underline" onClick={() => setGuide((v) => !v)}>
              {copy.gpuPanel}
            </button>
            {guide
              ? report.gpuGuide.map((line) => (
                  <p key={line} className="text-[11px] text-muted-foreground">
                    {line}
                  </p>
                ))
              : null}
          </div>
        ) : null}
      </div>

      <div className="flex shrink-0 items-center gap-2 border-t pt-2">
        <Button disabled={busy || selectedList.length === 0} onClick={requestApply}>
          {copy.apply} · {selectedList.length}
        </Button>
        <Button variant="outline" onClick={() => onRestore(null)} disabled={busy}>
          {copy.restoreAll}
        </Button>
        <span className="text-[11px] text-muted-foreground">
          {selectedList.length} {copy.selected}
        </span>
      </div>

      <Dialog open={confirmRisky} onOpenChange={setConfirmRisky}>
        <DialogContent>
          <DialogTitle>{copy.riskyTitle}</DialogTitle>
          <DialogDescription>{copy.riskyBody}</DialogDescription>
          <div className="mt-4 flex gap-2">
            <Button
              variant="destructive"
              onClick={() => {
                setConfirmRisky(false);
                onApply(selectedList, true, spoof || null);
              }}
            >
              {copy.riskyConfirm}
            </Button>
            <Button variant="outline" onClick={() => setConfirmRisky(false)}>
              {copy.riskyCancel}
            </Button>
          </div>
        </DialogContent>
      </Dialog>

      <Dialog open={restoreOpen} onOpenChange={setRestoreOpen}>
        <DialogContent>
          <DialogTitle>{copy.restorePickTitle}</DialogTitle>
          <DialogDescription>{copy.restoreSelectiveNote}</DialogDescription>
          <div className="mt-3 grid max-h-52 grid-cols-1 gap-2 overflow-y-auto sm:grid-cols-2">
            {restoreItems.map((item) => (
              <label key={item.id} className="flex items-start gap-2 text-sm">
                <Checkbox
                  className="mt-0.5"
                  checked={restorePick.has(item.id)}
                  disabled={!item.selective}
                  onCheckedChange={(value) => {
                    setRestorePick((cur) => {
                      const next = new Set(cur);
                      if (value === true) {
                        next.add(item.id);
                      } else {
                        next.delete(item.id);
                      }
                      return next;
                    });
                  }}
                />
                <span>
                  {item.name}
                  <span className="block text-xs text-muted-foreground">
                    {item.conflict ? copy.conflict : item.detail}
                  </span>
                </span>
              </label>
            ))}
          </div>
          <div className="mt-4 flex gap-2">
            <Button
              disabled={restorePick.size === 0}
              onClick={() => {
                setRestoreOpen(false);
                onRestore([...restorePick]);
              }}
            >
              {copy.restoreSelected}
            </Button>
            <Button variant="outline" onClick={() => setRestoreOpen(false)}>
              {copy.riskyCancel}
            </Button>
          </div>
        </DialogContent>
      </Dialog>
    </div>
  );
}
