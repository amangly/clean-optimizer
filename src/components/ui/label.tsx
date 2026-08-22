import { Label as LabelPrimitive } from "@radix-ui/react-label";
import type { ComponentProps } from "react";
import { cn } from "@/lib/utils";

export function Label({ className, ...props }: ComponentProps<typeof LabelPrimitive>) {
  return <LabelPrimitive className={cn("text-sm font-medium", className)} {...props} />;
}
