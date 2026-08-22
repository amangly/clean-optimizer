import logo from "@/assets/logo.png";
import { cn } from "@/lib/utils";

type Props = {
  className?: string;
};

export function Logo({ className }: Props) {
  return <img src={logo} alt="" width={40} height={40} className={cn("size-10 rounded-md", className)} />;
}
