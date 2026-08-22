import logo from "@/assets/logo.png";
import { cn } from "@/lib/utils";

type Props = {
  className?: string;
};

export function Logo({ className }: Props) {
  return (
    <img
      src={logo}
      alt=""
      width={32}
      height={32}
      className={cn("size-8 shrink-0 rounded-none bg-black object-cover", className)}
    />
  );
}
