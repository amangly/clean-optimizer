import { useEffect, useState } from "react";
import * as api from "@/lib/api";
import { copy } from "@/lib/copy";
import type { Candidate, ExperimentState } from "@/lib/types";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Alert } from "@/components/ui/alert";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";

export function TunePage() {
  const [scene, setScene] = useState("same-map-same-route");
  const [state, setState] = useState<ExperimentState | null>(null);
  const [library, setLibrary] = useState<Candidate[]>([]);
  const [avg, setAvg] = useState("120");
  const [low, setLow] = useState("80");
  const [hitches, setHitches] = useState("2");
  const [error, setError] = useState("");

  useEffect(() => {
    void api.experimentLibrary().then(setLibrary).catch((e: Error) => setError(e.message));
    void api.experimentStatus().then(setState).catch((e: Error) => setError(e.message));
  }, []);

  async function start() {
    setError("");
    try {
      setState(await api.startExperiment(scene));
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }

  async function confirm() {
    setError("");
    try {
      setState(await api.confirmExperimentRound(Number(avg), Number(low), Number(hitches)));
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }

  return (
    <div data-ui-scroll-container className="h-full min-h-0 space-y-3 overflow-y-auto text-sm">
      <p className="text-sm">{copy.tuneLead}</p>
      <div className="max-w-sm space-y-1">
        <Label>{copy.tuneScene}</Label>
        <Input value={scene} onChange={(e) => setScene(e.target.value)} />
      </div>
      <div className="flex flex-wrap gap-2">
        <Button onClick={() => void start()}>{copy.tuneStart}</Button>
        <Button variant="outline" onClick={() => void api.cancelExperiment().then(setState)}>
          {copy.tuneCancel}
        </Button>
      </div>
      {state ? (
        <p className="text-sm">
          {state.status} · baselines {state.baselineRuns}/3
          {state.currentGroup ? ` · group ${state.currentGroup}` : ""}
        </p>
      ) : null}
      <div className="flex flex-wrap items-end gap-2">
        <div className="space-y-1">
          <Label>Avg FPS</Label>
          <Input className="w-24" value={avg} onChange={(e) => setAvg(e.target.value)} />
        </div>
        <div className="space-y-1">
          <Label>1% low</Label>
          <Input className="w-24" value={low} onChange={(e) => setLow(e.target.value)} />
        </div>
        <div className="space-y-1">
          <Label>Hitches</Label>
          <Input className="w-24" value={hitches} onChange={(e) => setHitches(e.target.value)} />
        </div>
        <Button variant="outline" onClick={() => void confirm()}>
          {copy.tuneConfirm}
        </Button>
      </div>
      {library.map((c) => (
        <Card key={c.groupId} size="sm">
          <CardHeader>
            <CardTitle>
              {c.groupId} {c.displayName}
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-1 text-sm text-muted-foreground">
            <p>{c.purpose}</p>
            <p>{c.itemIds.join(", ")}</p>
          </CardContent>
        </Card>
      ))}
      {error ? <Alert>{error}</Alert> : null}
    </div>
  );
}
