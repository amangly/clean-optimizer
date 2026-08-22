import * as SelectPrimitive from "@radix-ui/react-select";
import { ChevronDown } from "lucide-react";
import type { ComponentProps } from "react";
import { cn } from "@/lib/utils";

export const Select = SelectPrimitive.Root;
export const SelectValue = SelectPrimitive.Value;

export function SelectTrigger({ className, children, ...props }: ComponentProps<typeof SelectPrimitive.Trigger>) {
  return (
    <SelectPrimitive.Trigger
      className={cn(
        "flex h-9 w-full items-center justify-between rounded-md border bg-transparent px-3 text-sm",
        className,
      )}
      {...props}
    >
      {children}
      <ChevronDown className="size-4 opacity-60" />
    </SelectPrimitive.Trigger>
  );
}

export function SelectContent({ className, children, ...props }: ComponentProps<typeof SelectPrimitive.Content>) {
  return (
    <SelectPrimitive.Portal>
      <SelectPrimitive.Content
        className={cn("z-50 overflow-hidden rounded-md border bg-popover shadow-md", className)}
        {...props}
      >
        <SelectPrimitive.Viewport className="p-1">{children}</SelectPrimitive.Viewport>
      </SelectPrimitive.Content>
    </SelectPrimitive.Portal>
  );
}

export function SelectItem({ className, children, ...props }: ComponentProps<typeof SelectPrimitive.Item>) {
  return (
    <SelectPrimitive.Item
      className={cn("relative flex cursor-default items-center rounded-sm px-2 py-1.5 text-sm outline-none data-[highlighted]:bg-accent", className)}
      {...props}
    >
      <SelectPrimitive.ItemText>{children}</SelectPrimitive.ItemText>
    </SelectPrimitive.Item>
  );
}
