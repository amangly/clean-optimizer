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
  const disabled = !item.applicable && item.kind !== "check";
  return (
    <label className="flex items-start gap-3 rounded-lg border px-3 py-2.5">
      <Checkbox
        className="mt-0.5"
        checked={checked}
        disabled={disabled}
        onCheckedChange={(value) => onToggle(item.id, value === true)}
      />
      <div className="min-w-0 flex-1">
        <div className="flex flex-wrap items-center gap-2">
          <h3 className="text-sm font-medium">{item.name}</h3>
          <Badge variant={item.tier === "risky" ? "warning" : item.optimized ? "default" : "outline"}>
            {item.tier === "risky" ? "Risky" : statusLabel(item.optimized, item.kind)}
            {item.reboot ? " · reboot" : ""}
          </Badge>
          {item.attention ? <Badge variant="warning">Check</Badge> : null}
        </div>
        <p className="mt-1 text-xs text-muted-foreground">{item.note}</p>
        {item.detail ? <TextLinks className="mt-1 block text-xs text-muted-foreground" text={item.detail} /> : null}
      </div>
    </label>
  );
}
