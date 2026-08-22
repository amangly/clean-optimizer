import { useMemo, useState } from "react";
import { ItemRow } from "@/components/ItemRow";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Checkbox } from "@/components/ui/checkbox";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { copy } from "@/lib/copy";
import type { DetectReport, ItemResult, RestoreItem, Preset } from "@/lib/types";

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
  const [confirmRisky, setConfirmRisky] = useState(false);
  const [restoreOpen, setRestoreOpen] = useState(false);
  const [restorePick, setRestorePick] = useState<Set<string>>(new Set());
  const [spoof, setSpoof] = useState(report.recommendedSpoof ?? report.spoofModels[0] ?? "");

  const selectedList = useMemo(() => [...selected], [selected]);
  const hasSpoof = selected.has("gpu-name-spoof");
  const preset = presets.find((p) => p.id === presetId);

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

  const selective = restoreItems.filter((i) => i.selective);

  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-end gap-2">
        <div className="min-w-[240px] flex-1 space-y-1">
          <Label>{copy.gamePath}</Label>
          <Input value={gamePath} onChange={(e) => onGamePath(e.target.value)} />
        </div>
        <Button variant="outline" onClick={onFind} disabled={busy}>
          {copy.find}
        </Button>
        <Button variant="outline" onClick={onBrowse} disabled={busy}>
          {copy.browse}
        </Button>
      </div>

      <div className="flex flex-wrap items-end gap-2">
        <div className="space-y-1">
          <Label>{copy.preset}</Label>
          <Select value={presetId} onValueChange={loadPreset}>
            <SelectTrigger className="w-[220px]">
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
        </div>
        <Button variant="outline" onClick={() => loadPreset(presetId)}>
          {copy.selectPreset}
        </Button>
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
          className="max-w-[180px]"
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
      </div>

      {preset?.note ? <p className="text-sm text-muted-foreground">{preset.note}</p> : null}

      {hasSpoof && report.spoofModels.length > 0 ? (
        <div className="max-w-sm space-y-1">
          <Label>{copy.spoofAs}</Label>
          <Select value={spoof} onValueChange={setSpoof}>
            <SelectTrigger>
              <SelectValue />
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

      <div className="space-y-2">
        {report.items.map((item) => (
          <ItemRow key={item.id} item={item} checked={selected.has(item.id)} onToggle={toggle} />
        ))}
      </div>

      <div className="flex flex-wrap gap-2">
        <Button disabled={busy || selectedList.length === 0} onClick={requestApply}>
          {copy.apply}
        </Button>
        <Button variant="outline" onClick={() => onRestore(null)} disabled={busy}>
          {copy.restoreAll}
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

      {lastApply ? (
        <div className="space-y-1">
          {lastApply.map((r) => (
            <div key={`${r.id}-${r.message}`} className="text-xs text-muted-foreground">
              {r.name}: {r.message}
            </div>
          ))}
        </div>
      ) : null}

      {report.gpuGuide.length ? (
        <div className="space-y-1">
          {report.gpuGuide.map((line) => (
            <p key={line} className="text-sm text-muted-foreground">
              {line}
            </p>
          ))}
        </div>
      ) : null}

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
          <div className="mt-3 max-h-64 space-y-2 overflow-y-auto">
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
