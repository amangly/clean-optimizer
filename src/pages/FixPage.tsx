import { useState } from "react";
import * as api from "@/lib/api";
import { copy } from "@/lib/copy";
import { formatBytes } from "@/lib/format";
import type { CacheReport, CheckResult } from "@/lib/types";
import { Button } from "@/components/ui/button";
import { Alert } from "@/components/ui/alert";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { TextLinks } from "@/components/text-links";

type Props = {
  gpuGuide: string[];
};

export function FixPage({ gpuGuide }: Props) {
  const [checks, setChecks] = useState<CheckResult[]>([]);
  const [cache, setCache] = useState<CacheReport | null>(null);
  const [diag, setDiag] = useState("");
  const [error, setError] = useState("");

  async function run() {
    setError("");
    try {
      setChecks(await api.runChecks());
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }

  async function wipe() {
    setError("");
    try {
      setCache(await api.cleanShaderCache());
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }

  return (
    <div data-ui-scroll-container className="h-full min-h-0 space-y-3 overflow-y-auto text-sm">
      <p className="text-sm">{copy.fixLead}</p>
      <div className="flex flex-wrap gap-2">
        <Button onClick={() => void run()}>{copy.runChecks}</Button>
        <Button variant="destructive" onClick={() => void wipe()}>
          {copy.clearCache}
        </Button>
        <Button
          variant="outline"
          onClick={() => {
            setError("");
            void api
              .diagnose()
              .then(setDiag)
              .catch((e: Error) => setError(e.message));
          }}
        >
          {copy.diagnose}
        </Button>
      </div>
      {gpuGuide.length > 0 ? (
        <Card size="sm">
          <CardHeader>
            <CardTitle>{copy.gpuPanel}</CardTitle>
          </CardHeader>
          <CardContent className="space-y-1 text-sm text-muted-foreground">
            {gpuGuide.map((line) => (
              <p key={line}>{line}</p>
            ))}
          </CardContent>
        </Card>
      ) : null}
      {checks.map((c) => (
        <Card key={c.id} size="sm">
          <CardHeader>
            <CardTitle>{c.id}</CardTitle>
          </CardHeader>
          <CardContent>
            {c.attention ? <Alert>{c.text}</Alert> : <TextLinks className="text-sm text-muted-foreground" text={c.text} />}
          </CardContent>
        </Card>
      ))}
      {cache ? (
        <p className="text-sm">
          Deleted {cache.deletedFiles} files ({formatBytes(cache.bytes)}). Skipped {cache.skipped}.
        </p>
      ) : null}
      {diag ? <pre className="whitespace-pre-wrap text-xs text-muted-foreground">{diag}</pre> : null}
      {error ? <Alert>{error}</Alert> : null}
    </div>
  );
}
