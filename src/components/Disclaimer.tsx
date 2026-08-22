import { useEffect, useRef, useState } from "react";
import { copy } from "@/lib/copy";
import { reachedEnd } from "@/lib/disclaimer-scroll";
import { Button } from "@/components/ui/button";
import { Logo } from "@/components/Logo";

type Props = {
  onAccept: () => void;
  onQuit: () => void;
};

export function Disclaimer({ onAccept, onQuit }: Props) {
  const scroller = useRef<HTMLDivElement>(null);
  const [ready, setReady] = useState(false);

  function measure() {
    const el = scroller.current;
    if (!el) {
      return;
    }
    setReady(reachedEnd(el.scrollTop, el.clientHeight, el.scrollHeight));
  }

  useEffect(() => {
    measure();
    const el = scroller.current;
    if (!el) {
      return;
    }
    const observer = new ResizeObserver(() => measure());
    observer.observe(el);
    return () => observer.disconnect();
  }, []);

  return (
    <div className="mx-auto flex h-dvh max-w-xl flex-col gap-4 px-6 py-8">
      <div className="flex items-center gap-3">
        <Logo />
        <h1 className="text-lg font-semibold">{copy.disclaimerTitle}</h1>
      </div>
      <div
        ref={scroller}
        data-ui-scroll-container
        className="min-h-0 flex-1 overflow-y-auto rounded-xl border bg-card p-4"
        onScroll={measure}
      >
        <div className="ui-selectable space-y-3 text-sm leading-6">
          {copy.disclaimerBody.map((p) => (
            <p key={p}>{p}</p>
          ))}
        </div>
      </div>
      <p className="text-sm text-muted-foreground">{ready ? copy.scrolled : copy.scrollHint}</p>
      <div className="flex gap-2">
        <Button disabled={!ready} onClick={onAccept}>
          {copy.accept}
        </Button>
        <Button variant="outline" onClick={onQuit}>
          {copy.quit}
        </Button>
      </div>
    </div>
  );
}
