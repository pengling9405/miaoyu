"use client";

import SunIcon from "@heroicons/react/24/outline/SunIcon";
import MoonIcon from "@heroicons/react/24/solid/MoonIcon";
import { Switch } from "~/components/animate/switch";
import { useTheme } from "~/components/theme/provider";

export const ThemeSwitch = ({ className }: { className?: string }) => {
	const { theme, setTheme } = useTheme();
	return (
		<Switch
			leftIcon={<SunIcon />}
			rightIcon={<MoonIcon />}
			className={className}
			checked={theme === "dark"}
			onCheckedChange={(checked) => setTheme(checked ? "dark" : "light")}
		/>
	);
};
