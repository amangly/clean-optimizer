import { useEffect, useState } from "react";
import * as api from "@/lib/api";
import { copy } from "@/lib/copy";

export function LogPage() {
  const [text, setText] = useState("");

  useEffect(() => {
    void api.readLog().then(setText);
  }, []);

  return (
    <pre className="ui-selectable whitespace-pre-wrap rounded-xl border bg-card p-4 text-xs leading-5">
      {text || copy.logEmpty}
    </pre>
  );
}
