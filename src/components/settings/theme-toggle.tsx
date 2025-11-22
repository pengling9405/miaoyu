import ComputerDesktopIcon from "@heroicons/react/24/outline/ComputerDesktopIcon";
import MoonIcon from "@heroicons/react/24/outline/MoonIcon";
import SunIcon from "@heroicons/react/24/outline/SunIcon";
import {
	ToggleGroup,
	ToggleGroupItem,
} from "~/components/animate-ui/components/radix/toggle-group";
import { useTheme } from "~/components/theme/provider";
import type { AppTheme } from "~/lib/tauri";
import { cn } from "~/lib/utils";

type ThemeToggleProps = {
	className?: string;
	size?: "sm" | "default";
};

const sizeConfig = {
	sm: {
		group: "sm",
		item: "icon" as const,
	},
	default: {
		group: "default",
		item: "default" as const,
	},
} as const;

export function ThemeToggle({ className, size = "sm" }: ThemeToggleProps) {
	const { theme, setTheme } = useTheme();
	const resolvedSize = sizeConfig[size];

	return (
		<ToggleGroup
			size={resolvedSize.group}
			type="single"
			variant="outline"
			value={theme as AppTheme}
			onValueChange={(value) => value && setTheme(value as AppTheme)}
			className={cn(
				"rounded-xl border border-border/60 bg-muted/30",
				className,
			)}
		>
			<ToggleGroupItem
				size={resolvedSize.item}
				value="system"
				aria-label="自动跟随系统"
			>
				<ComputerDesktopIcon className="size-4" />
			</ToggleGroupItem>
			<ToggleGroupItem
				size={resolvedSize.item}
				value="light"
				aria-label="浅色主题"
			>
				<SunIcon className="size-4" />
			</ToggleGroupItem>
			<ToggleGroupItem
				size={resolvedSize.item}
				value="dark"
				aria-label="深色主题"
			>
				<MoonIcon className="size-4" />
			</ToggleGroupItem>
		</ToggleGroup>
	);
}
