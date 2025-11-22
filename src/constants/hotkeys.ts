import type { Hotkey } from "~/lib/tauri";

export const DEFAULT_DICTATION_HOTKEY: Hotkey = {
	code: "Space",
	meta: false,
	ctrl: false,
	alt: true,
	shift: false,
};

export const DEFAULT_DIARY_HOTKEY: Hotkey = {
	code: "Space",
	meta: false,
	ctrl: false,
	alt: true,
	shift: true,
};
