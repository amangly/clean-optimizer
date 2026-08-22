import type { HTMLAttributes } from "react";
import { cn } from "@/lib/utils";

export function Alert({ className, ...props }: HTMLAttributes<HTMLDivElement>) {
  return (
    <div
      className={cn("rounded-lg border px-3 py-2 text-sm text-amber-200 border-amber-500/40 bg-amber-500/10", className)}
      {...props}
    />
  );
}
