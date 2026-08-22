import { useState } from "react";
import { Checkbox } from "@/components/ui/checkbox";
import { Badge } from "@/components/ui/badge";
import { TextLinks } from "@/components/text-links";
import { statusLabel } from "@/lib/format";
import type { ItemView } from "@/lib/types";

type Props = {
  item: ItemView;
  checked: boolean;
  onToggle: (id: string, next: boolean) => void;
};

export function ItemRow({ item, checked, onToggle }: Props) {
  const [open, setOpen] = useState(false);
  const disabled = !item.applicable && item.kind !== "check";
  const status = item.tier === "risky" ? "Risky" : statusLabel(item.optimized, item.kind);
  return (
    <div className="border-b last:border-0">
      <div className="flex items-center gap-2 py-1">
        <Checkbox
          checked={checked}
          disabled={disabled}
          onCheckedChange={(value) => onToggle(item.id, value === true)}
        />
        <button
          type="button"
          className="min-w-0 flex-1 truncate text-left text-xs"
          onClick={() => setOpen((v) => !v)}
        >
          {item.name}
        </button>
        <Badge variant={item.attention || item.tier === "risky" ? "destructive" : "outline"}>
          {item.attention ? "Check" : status}
          {item.reboot ? " · reboot" : ""}
        </Badge>
      </div>
      {open ? (
        <div className="ui-selectable pb-2 pl-6 text-xs text-muted-foreground">
          <p>{item.note}</p>
          {item.detail ? <TextLinks className="mt-1 block" text={item.detail} /> : null}
        </div>
      ) : null}
    </div>
  );
}
