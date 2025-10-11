import { ScriptOnce } from "@tanstack/react-router";
import { createClientOnlyFn, createIsomorphicFn } from "@tanstack/react-start";
import {
	createContext,
	type ReactNode,
	use,
	useCallback,
	useEffect,
	useLayoutEffect,
	useMemo,
	useState,
} from "react";
import { z } from "zod";

import { commands } from "~/lib/tauri";
import { settingsStore } from "~/store";

const ThemeSchema = z.enum(["light", "dark", "system"]).catch("system");

export type Theme = z.infer<typeof ThemeSchema>;

const themeStorageKey = "ui-theme";

const getStoredTheme = createIsomorphicFn()
	.server((): Theme => "system")
	.client((): Theme => {
		const stored = localStorage.getItem(themeStorageKey);
		return ThemeSchema.parse(stored);
	});

const resolveTheme = (theme: Theme): "light" | "dark" => {
	if (theme === "system") {
		if (typeof window !== "undefined") {
			return window.matchMedia("(prefers-color-scheme: dark)").matches
				? "dark"
				: "light";
		}
		return "light";
	}
	return theme;
};

const applyTheme = createClientOnlyFn((theme: Theme) => {
	const validatedTheme = ThemeSchema.parse(theme);
	const root = document.documentElement;
	const resolved = resolveTheme(validatedTheme);
	root.classList.remove("light", "dark");
	root.classList.add(resolved);
	localStorage.setItem(themeStorageKey, validatedTheme);
});

const themeScript = (() => {
	function themeFn() {
		try {
			const storedTheme = localStorage.getItem("ui-theme");
			if (storedTheme === "dark" || storedTheme === "light") {
				document.documentElement.classList.remove("light", "dark");
				document.documentElement.classList.add(storedTheme);
				return;
			}

			const prefersDark = window.matchMedia?.(
				"(prefers-color-scheme: dark)",
			).matches;
			document.documentElement.classList.remove("light", "dark");
			document.documentElement.classList.add(prefersDark ? "dark" : "light");
		} catch {
			document.documentElement.classList.add("light");
		}
	}
	return `(${themeFn.toString()})();`;
})();

type ThemeContextProps = {
	theme: Theme;
	setTheme: (theme: Theme) => void;
};

const ThemeContext = createContext<ThemeContextProps | undefined>(undefined);

type ThemeProviderProps = {
	children: ReactNode;
};

export function ThemeProvider({ children }: ThemeProviderProps) {
	const settingsQuery = settingsStore.useQuery();

	const [theme, setThemeState] = useState<Theme>(() => {
		if (typeof window !== "undefined") {
			return getStoredTheme();
		}
		return "system";
	});

	useEffect(() => {
		const storeTheme = settingsQuery.data?.theme;
		if (storeTheme && storeTheme !== theme) {
			setThemeState(ThemeSchema.parse(storeTheme));
		}
	}, [settingsQuery.data?.theme, theme]);

	useLayoutEffect(() => {
		applyTheme(theme);

		// 同步更新 Tauri 窗口主题（确保窗口装饰立即更新）
		void commands.setTheme(theme).catch(() => {
			/* ignore dev preview without tauri */
		});
	}, [theme]);

	useEffect(() => {
		if (typeof window === "undefined" || theme !== "system") {
			return;
		}
		const media = window.matchMedia("(prefers-color-scheme: dark)");
		const handler = () => {
			applyTheme("system");
			void commands.setTheme("system").catch(() => {
				/* ignore dev preview without tauri */
			});
		};
		media.addEventListener("change", handler);
		return () => media.removeEventListener("change", handler);
	}, [theme]);

	const setTheme = useCallback(
		(newTheme: Theme) => {
			const validatedTheme = ThemeSchema.parse(newTheme);
			setThemeState(validatedTheme);
			void settingsQuery.set({ theme: validatedTheme });
		},
		[settingsQuery],
	);

	const contextValue = useMemo(() => ({ theme, setTheme }), [theme, setTheme]);

	return (
		<ThemeContext value={contextValue}>
			<ScriptOnce>{themeScript}</ScriptOnce>
			{children}
		</ThemeContext>
	);
}

export const useTheme = () => {
	const context = use(ThemeContext);
	if (!context) {
		throw new Error("useTheme must be used within a ThemeProvider");
	}
	return context;
};
