import { Check, Copy } from "lucide-react";
import * as React from "react";
import { Button } from "~/components/ui/button";
import { cn } from "~/lib/utils";

async function copyToClipboardWithMeta(value: string) {
	if (!navigator?.clipboard?.writeText) {
		throw new Error("当前环境不支持复制到剪贴板");
	}
	await navigator.clipboard.writeText(value);
}

type CopyButtonProps = React.ComponentProps<typeof Button> & {
	value: string;
	tooltip?: string;
	onCopySuccess?: () => void | Promise<void>;
	onCopyError?: (error: Error) => void | Promise<void>;
};

export function CopyButton({
	value,
	className,
	variant = "text",
	tooltip = "复制到剪贴板",
	onClick,
	onCopySuccess,
	onCopyError,
	...buttonProps
}: CopyButtonProps) {
	const [hasCopied, setHasCopied] = React.useState(false);

	React.useEffect(() => {
		if (!hasCopied) {
			return;
		}
		const timer = window.setTimeout(() => {
			setHasCopied(false);
		}, 2000);
		return () => {
			window.clearTimeout(timer);
		};
	}, [hasCopied]);

	const handleCopy = async () => {
		await copyToClipboardWithMeta(value);
		setHasCopied(true);
	};

	const handleClick = async (
		event: React.MouseEvent<HTMLButtonElement, MouseEvent>,
	) => {
		onClick?.(event);
		try {
			await handleCopy();
			await onCopySuccess?.();
		} catch (error) {
			const err = error instanceof Error ? error : new Error(String(error));
			await onCopyError?.(err);
		}
	};

	return (
		<Button
			data-slot="copy-button"
			data-copied={hasCopied}
			size="xs"
			variant={variant}
			className={cn(
				"text-xs gap-1 text-muted-foreground hover:text-accent-foreground",
				className,
			)}
			onClick={handleClick}
			{...buttonProps}
		>
			{hasCopied ? <Check className="size-3" /> : <Copy className="size-3" />}
			<span>复制</span>
		</Button>
	);
}
