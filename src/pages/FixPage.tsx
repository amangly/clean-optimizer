import { useState } from "react";
import * as api from "@/lib/api";
import { copy } from "@/lib/copy";
import { formatBytes } from "@/lib/format";
import type { CacheReport, CheckResult } from "@/lib/types";
import { Button } from "@/components/ui/button";
import { Alert } from "@/components/ui/alert";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { TextLinks } from "@/components/text-links";

export function FixPage() {
  const [checks, setChecks] = useState<CheckResult[]>([]);
  const [cache, setCache] = useState<CacheReport | null>(null);
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
    <div className="space-y-4">
      <p className="text-sm">{copy.fixLead}</p>
      <div className="flex flex-wrap gap-2">
        <Button onClick={() => void run()}>{copy.runChecks}</Button>
        <Button variant="destructive" onClick={() => void wipe()}>
          {copy.clearCache}
        </Button>
      </div>
      {checks.map((c) => (
        <Card key={c.id}>
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
      {error ? <Alert>{error}</Alert> : null}
    </div>
  );
}
