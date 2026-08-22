import { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import * as api from "@/lib/api";
import { copy } from "@/lib/copy";

export function LogPage() {
  const [text, setText] = useState("");
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    let live = true;
    function pull() {
      void api.readLog().then((next) => {
        if (live) {
          setText(next);
        }
      });
    }
    pull();
    const id = window.setInterval(pull, 1000);
    return () => {
      live = false;
      window.clearInterval(id);
    };
  }, []);

  return (
    <div className="flex h-full min-h-0 flex-col gap-2">
      <div className="flex shrink-0 items-center">
        <Button
          variant="outline"
          disabled={!text}
          onClick={() => {
            void navigator.clipboard.writeText(text).then(() => {
              setCopied(true);
              window.setTimeout(() => setCopied(false), 1200);
            });
          }}
        >
          {copied ? copy.logCopied : copy.copyLog}
        </Button>
      </div>
      <pre
        data-ui-scroll-container
        className="ui-selectable min-h-0 flex-1 overflow-y-auto whitespace-pre-wrap rounded-none border bg-card p-3 text-xs leading-5"
      >
        {text || copy.logEmpty}
      </pre>
    </div>
  );
}
