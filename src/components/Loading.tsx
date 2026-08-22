import { Spinner } from "@/components/ui/spinner";

type Props = {
  label: string;
};

export function Loading({ label }: Props) {
  return (
    <div className="grid h-dvh place-items-center">
      <div className="flex items-center gap-2 text-xs text-muted-foreground">
        <Spinner className="size-4" />
        <span>{label}</span>
      </div>
    </div>
  );
}
