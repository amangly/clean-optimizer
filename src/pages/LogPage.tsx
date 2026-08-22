import { useEffect, useState } from "react";
import * as api from "@/lib/api";
import { copy } from "@/lib/copy";

export function LogPage() {
  const [text, setText] = useState("");

  useEffect(() => {
    void api.readLog().then(setText);
  }, []);

  return (
    <pre data-ui-scroll-container className="ui-selectable h-full min-h-0 overflow-y-auto whitespace-pre-wrap rounded-none border bg-card p-3 text-xs leading-5">      {text || copy.logEmpty}
    </pre>
  );
}
