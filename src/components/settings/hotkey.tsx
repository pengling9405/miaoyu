import CheckIcon from "@heroicons/react/24/outline/CheckIcon";
import XMarkIcon from "@heroicons/react/24/solid/XMarkIcon";
import { Fragment, useEffect, useMemo, useState } from "react";
import { DEFAULT_DICTATION_HOTKEY } from "~/constants/hotkeys";
import type { Hotkey } from "~/lib/tauri";
import { cn } from "~/lib/utils";

type Props = {
	currentHotkey?: Hotkey;
	defaultHotkey?: Hotkey;
	onUpdate: (hotkey: Hotkey | null) => Promise<void>;
};

export function HotkeySetting({
	currentHotkey,
	onUpdate,
	defaultHotkey,
}: Props) {
	const [mode, setMode] = useState<"view" | "listening" | "confirm">("view");
	const [draftHotkey, setDraftHotkey] = useState<Hotkey | null>(null);
	const [saving, setSaving] = useState(false);

	useEffect(() => {
		if (mode !== "listening") return;

		const handleKeyDown = (event: KeyboardEvent) => {
			if (MODIFIER_CODES.has(event.code)) {
				return;
			}

			event.preventDefault();
			event.stopPropagation();

			if (event.code === "Escape") {
				setMode("view");
				setDraftHotkey(null);
				return;
			}

			const nextHotkey: Hotkey = {
				code: event.code,
				meta: event.metaKey,
				ctrl: event.ctrlKey,
				alt: event.altKey,
				shift: event.shiftKey,
			};

			setDraftHotkey(nextHotkey);
			setMode("confirm");
		};

		const handleBlur = () => {
			setMode("view");
			setDraftHotkey(null);
		};

		window.addEventListener("keydown", handleKeyDown, true);
		window.addEventListener("blur", handleBlur);

		return () => {
			window.removeEventListener("keydown", handleKeyDown, true);
			window.removeEventListener("blur", handleBlur);
		};
	}, [mode]);

	const displayHotkey = useMemo(() => {
		if (mode === "confirm" && draftHotkey) {
			return draftHotkey;
		}
		return currentHotkey ?? defaultHotkey ?? DEFAULT_DICTATION_HOTKEY;
	}, [mode, draftHotkey, currentHotkey, defaultHotkey]);

	const startListening = () => {
		setDraftHotkey(null);
		setMode("listening");
	};

	const cancelEditing = () => {
		setDraftHotkey(null);
		setSaving(false);
		setMode("view");
	};

	const confirmHotkey = async () => {
		if (!draftHotkey) {
			setMode("view");
			return;
		}

		if (currentHotkey && hotkeysEqual(draftHotkey, currentHotkey)) {
			setMode("view");
			setDraftHotkey(null);
			return;
		}

		setSaving(true);
		try {
			await onUpdate(draftHotkey);
			setMode("view");
			setDraftHotkey(null);
		} catch (error) {
			console.error(error);
		} finally {
			setSaving(false);
		}
	};

	const showCancel = mode !== "view";
	const showConfirm = mode === "confirm";

	return (
		<div className="relative flex items-center">
			<button
				type="button"
				className={cn(
					"rounded-lg border border-border/60 bg-card p-1 text-xs flex items-center gap-1 transition-colors hover:border-border",
					mode === "listening" && "px-3",
				)}
				onClick={mode === "view" ? startListening : undefined}
				aria-live="polite"
				disabled={mode === "confirm" && saving}
			>
				{mode === "listening" ? (
					<span className="text-sm">请按快捷键组合</span>
				) : (
					<HotkeyVisual hotkey={displayHotkey} />
				)}
			</button>

			{showCancel && (
				<button
					type="button"
					className="absolute -left-2.5 -top-2.5 flex h-5 w-5 items-center justify-center rounded-full bg-muted text-xs text-muted-foreground shadow"
					onClick={cancelEditing}
					aria-label="取消设置快捷键"
				>
					<XMarkIcon className="size-2.5" />
				</button>
			)}

			{showConfirm && (
				<button
					type="button"
					className="absolute -right-2.5 -top-2.5 flex h-5 w-5 items-center justify-center rounded-full bg-primary text-primary-foreground text-xs shadow disabled:opacity-60"
					onClick={confirmHotkey}
					disabled={saving}
					aria-label="保存快捷键"
				>
					<CheckIcon className="size-2.5" />
				</button>
			)}
		</div>
	);
}

const MODIFIER_CODES = new Set([
	"ShiftLeft",
	"ShiftRight",
	"ControlLeft",
	"ControlRight",
	"AltLeft",
	"AltRight",
	"MetaLeft",
	"MetaRight",
]);

function HotkeyVisual({ hotkey }: { hotkey: Hotkey }) {
	const parts = toHotkeyParts(hotkey);

	return (
		<div className="flex items-center gap-1">
			{parts
				.map((part) => (
					<Fragment key={part}>
						<span className="rounded-md bg-secondary/70 px-2 py-1 text-xs font-medium text-secondary-foreground">
							{part}
						</span>
					</Fragment>
				))
				.reduce<React.ReactNode[]>((acc, curr, index) => {
					if (index > 0) {
						acc.push(
							<span
								key={`sep-${index}`}
								className="text-xs text-muted-foreground"
							>
								+
							</span>,
						);
					}
					acc.push(curr);
					return acc;
				}, [])}
		</div>
	);
}

function toHotkeyParts(hotkey: Hotkey): string[] {
	const parts: string[] = [];

	if (hotkey.meta) parts.push("Command");
	if (hotkey.ctrl) parts.push("Control");
	if (hotkey.alt) parts.push("Option");
	if (hotkey.shift) parts.push("Shift");

	parts.push(codeToLabel(hotkey.code));

	return parts;
}

function codeToLabel(code: string): string {
	if (code.startsWith("Key")) return code.slice(3);
	if (code.startsWith("Digit")) return code.slice(5);
	const specialMap: Record<string, string> = {
		Space: "Space",
		Enter: "Enter",
		Escape: "Esc",
		Backspace: "Backspace",
		Tab: "Tab",
		ArrowUp: "ArrowUp",
		ArrowDown: "ArrowDown",
		ArrowLeft: "ArrowLeft",
		ArrowRight: "ArrowRight",
	};
	return specialMap[code] ?? code;
}

function hotkeysEqual(a?: Hotkey | null, b?: Hotkey | null) {
	if (!a && !b) return true;
	if (!a || !b) return false;
	return (
		a.code === b.code &&
		a.meta === b.meta &&
		a.ctrl === b.ctrl &&
		a.alt === b.alt &&
		a.shift === b.shift
	);
}
