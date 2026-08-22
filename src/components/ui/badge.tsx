import type { HTMLAttributes } from "react";
import { cn } from "@/lib/utils";

type Props = HTMLAttributes<HTMLSpanElement> & {
  variant?: "default" | "secondary" | "outline" | "warning";
};

export function Badge({ className, variant = "outline", ...props }: Props) {
  return (
    <span
      className={cn(
        "inline-flex items-center rounded-md border px-1.5 py-0.5 text-[11px]",
        variant === "default" && "border-transparent bg-primary text-primary-foreground",
        variant === "secondary" && "border-transparent bg-secondary text-secondary-foreground",
        variant === "warning" && "border-transparent bg-amber-500/15 text-amber-500",
        className,
      )}
      {...props}
    />
  );
}
