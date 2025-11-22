import { useQuery } from "@tanstack/react-query";
import { createFileRoute } from "@tanstack/react-router";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
	ArrowDownToLine,
	Gauge,
	Lock,
	Mic,
	PersonStanding,
	Zap,
} from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { Logo } from "~/components/layouts/dashboard/logo";
import { Button } from "~/components/ui/button";
import { Card, CardContent } from "~/components/ui/card";
import { Kbd, KbdGroup } from "~/components/ui/kbd";
import { Progress } from "~/components/ui/progress";
import { Spinner } from "~/components/ui/spinner";
import { Textarea } from "~/components/ui/textarea";
import { DEFAULT_DICTATION_HOTKEY } from "~/constants/hotkeys";
import { PARAFORMER_MODEL_ID } from "~/constants/models";
import {
	useCheckPermissions,
	useOpenPermissionSettings,
	useRequestPermission,
} from "~/hooks/use-permissions";
import { commands, type Hotkey, type OSPermission } from "~/lib/tauri";
import { cn } from "~/lib/utils";
import { hotkeysStore } from "~/store";

type OfflineDownloadProgressEvent = {
	modelId: string;
	receivedBytes: number;
	totalBytes: number | null;
};

const PERMISSIONS: Array<{
	key: OSPermission;
	title: string;
	description: string;
	icon: typeof Mic;
	action: string;
}> = [
	{
		key: "microphone",
		title: "麦克风权限",
		description: "妙语需要使用麦克风来录制您的语音并进行识别。",
		icon: Mic,
		action: "允许访问",
	},
	{
		key: "accessibility",
		title: "辅助功能权限",
		description: "妙语需要辅助功能将转写内容自动填入输入框。",
		icon: PersonStanding,
		action: "打开设置",
	},
];

const KEY_FEATURES = [
	{
		icon: Zap,
		title: "近乎实时的转写",
		description: "低延迟，文本几乎瞬时完成转写。",
	},
	{
		icon: Gauge,
		title: "轻量资源占用",
		description: "CPU、内存极低占用。",
	},
	{
		icon: Lock,
		title: "私有且离线",
		description: "音频全部处理都在本地完成。",
	},
];

export const Route = createFileRoute("/onboarding")({
	component: RouteComponent,
});

type OnboardingStep = "download" | "permissions";
function RouteComponent() {
	const [currentStep, setCurrentStep] = useState<OnboardingStep>("download");
	const [downloading, setDownloading] = useState(false);
	const [downloadError, setDownloadError] = useState<string | null>(null);
	const [downloadState, setDownloadState] = useState<{
		percent: number;
		receivedBytes: number;
		totalBytes: number | null;
	} | null>(null);
	const hasClosedRef = useRef(false);

	const { data: offlineStatus, refetch: refetchOfflineStatus } = useQuery({
		queryKey: ["offline-model-status", "onboarding"],
		queryFn: () => commands.getOfflineModelsStatus(),
	});
	const { data: supportedModels } = useQuery({
		queryKey: ["supported-models"],
		queryFn: () => commands.getSupportedModels(),
	});
	const paraformerModel = supportedModels?.asrModels?.find(
		(model) => model.id === PARAFORMER_MODEL_ID,
	);
	const paraformerSizeLabel = paraformerModel?.size ?? "650 MB";
	const paraformerStatus = offlineStatus?.models?.find(
		(model) => model.id === PARAFORMER_MODEL_ID,
	);
	const paraformerReady = Boolean(paraformerStatus?.ready);

	const { data: permissionsStatus, refetch: refetchPermissions } =
		useCheckPermissions(false);
	const requestPermission = useRequestPermission();
	const openPermissionSettings = useOpenPermissionSettings();
	const hotkeysQuery = hotkeysStore.useQuery();
	const dictationHotkey =
		hotkeysQuery.data?.hotkeys?.startDictating ?? DEFAULT_DICTATION_HOTKEY;

	const microphoneStatus = permissionsStatus?.microphone;
	const accessibilityStatus = permissionsStatus?.accessibility;
	const microphoneReady =
		microphoneStatus === "granted" || microphoneStatus === "notNeeded";
	const accessibilityReady =
		accessibilityStatus === "granted" || accessibilityStatus === "notNeeded";
	const permissionsReady = microphoneReady && accessibilityReady;
	const canFinish = paraformerReady && permissionsReady;

	useEffect(() => {
		let unlisten: (() => void) | null = null;
		listen<OfflineDownloadProgressEvent>(
			"offline-model-download-progress",
			(event) => {
				if (event.payload.modelId !== PARAFORMER_MODEL_ID) {
					return;
				}
				setDownloadState({
					percent: event.payload.totalBytes
						? Math.min(
								100,
								Math.round(
									(event.payload.receivedBytes / event.payload.totalBytes) *
										100,
								),
							)
						: 0,
					receivedBytes: event.payload.receivedBytes,
					totalBytes: event.payload.totalBytes,
				});
			},
		)
			.then((unlistenFn) => {
				unlisten = unlistenFn;
			})
			.catch((error) => console.error(error));

		return () => {
			unlisten?.();
		};
	}, []);

	useEffect(() => {
		if (paraformerReady && currentStep === "download") {
			setCurrentStep("permissions");
		}
	}, [paraformerReady, currentStep]);

	useEffect(() => {
		const interval = window.setInterval(() => {
			refetchPermissions().catch(() => {});
		}, 1500);
		return () => {
			window.clearInterval(interval);
		};
	}, [refetchPermissions]);

	const handleDownload = async () => {
		setDownloadError(null);
		setDownloading(true);
		setDownloadState(null);
		try {
			await commands.downloadOfflineModels(PARAFORMER_MODEL_ID);
			await refetchOfflineStatus();
		} catch (error) {
			setDownloadError(error instanceof Error ? error.message : String(error));
		} finally {
			setDownloading(false);
		}
	};

	const handlePermissionAction = (
		permission: OSPermission,
		status: string | undefined,
	) => {
		if (status === "denied") {
			openPermissionSettings.mutate(permission);
		} else {
			requestPermission.mutate(permission);
		}
	};

	const handleFinish = async () => {
		if (!canFinish || hasClosedRef.current) return;
		hasClosedRef.current = true;
		try {
			await commands.setOnboardingCompleted(true);
			const window = getCurrentWindow();
			await window.close();
		} catch (error) {
			console.error("Failed to close setup window:", error);
			hasClosedRef.current = false;
		}
	};

	const hotkeyParts = useMemo(
		() => formatHotkeyParts(dictationHotkey),
		[dictationHotkey],
	);

	const downloadPercent = paraformerReady ? 100 : (downloadState?.percent ?? 0);
	const receivedLabel = downloadState
		? formatBytes(downloadState.receivedBytes)
		: paraformerReady
			? paraformerSizeLabel
			: "0 MB";
	const totalLabel =
		typeof downloadState?.totalBytes === "number" &&
		downloadState.totalBytes > 0
			? formatBytes(downloadState.totalBytes)
			: paraformerSizeLabel;

	const showPermissions = currentStep === "permissions";

	return (
		<div className="flex min-h-screen items-center justify-center bg-background py-12">
			<div className="w-full max-w-md">
				<div className="flex flex-col items-center text-center">
					<OnboardingHeader />
				</div>
				{showPermissions ? (
					<div className="mt-8 flex flex-col gap-6 text-left">
						<PermissionsSetup
							accessibilityStatus={accessibilityStatus}
							hotkeyParts={hotkeyParts}
							microphoneStatus={microphoneStatus}
							onPermissionAction={handlePermissionAction}
							canFinish={canFinish}
							permissionsReady={permissionsReady}
							handleFinish={handleFinish}
						/>
					</div>
				) : (
					<div className="mt-10 space-y-6">
						<FeaturesCard />
						<DownloadASRModel
							downloadError={downloadError}
							downloadPercent={downloadPercent}
							downloading={downloading}
							onDownload={handleDownload}
							onContinue={() => setCurrentStep("permissions")}
							ready={paraformerReady}
							sizeLabel={paraformerSizeLabel}
							receivedLabel={receivedLabel}
							totalLabel={totalLabel}
						/>
					</div>
				)}
			</div>
		</div>
	);
}

function OnboardingHeader() {
	return (
		<header className="flex flex-col items-center gap-4 text-center">
			<div className="flex items-center gap-4">
				<div className="rounded-lg bg-primary/10 p-1 text-primary">
					<Logo className="size-10" />
				</div>
				<p className="text-3xl font-semibold tracking-tight">妙语</p>
			</div>
			<div className="space-y-1">
				<p className="text-sm text-muted-foreground">
					智能语音输入，妙语亦可生花。
				</p>
			</div>
		</header>
	);
}

function FeaturesCard() {
	return (
		<Card className="p-4">
			<CardContent className="px-0">
				{KEY_FEATURES.map((feature) => (
					<div key={feature.title} className="flex items-start gap-3 py-3">
						<div className="rounded-full p-2 bg-secondary text-primary">
							<feature.icon className="size-5" />
						</div>
						<div className="space-y-1">
							<p className="text-sm font-semibold text-foreground">
								{feature.title}
							</p>
							<p className="text-xs text-muted-foreground">
								{feature.description}
							</p>
						</div>
					</div>
				))}
			</CardContent>
		</Card>
	);
}

type DownloadASRModelProps = {
	downloadError: string | null;
	downloadPercent: number;
	downloading: boolean;
	onDownload: () => void;
	onContinue: () => void;
	ready: boolean;
	receivedLabel: string;
	totalLabel: string;
	sizeLabel: string;
};

function DownloadASRModel({
	downloadPercent,
	downloading,
	onDownload,
	sizeLabel,
}: DownloadASRModelProps) {
	return (
		<section className="flex flex-col gap-8">
			<div className="flex flex-col gap-2 rounded-xl border p-4">
				<div className="flex flex-col gap-2">
					<p className="text-xl font-semibold text-foreground">
						下载离线语音识别模型
					</p>
					<p className="text-sm text-muted-foreground">
						妙语会把模型保存在你的电脑，本地完成转写，音频不经云端。
					</p>
				</div>
				<div className="flex flex-row items-center justify-between mt-2 text-xs text-muted-foreground">
					<p>{`${sizeLabel} · 仅需下载一次，即可离线使用。`}</p>
					{downloading && <p>{downloadPercent}%</p>}
				</div>
				<Progress
					value={downloadPercent}
					className={cn(!downloading && "bg-transparent")}
				/>
			</div>
			<div className="flex items-center justify-center">
				<Button
					size="lg"
					className="rounded-full"
					disabled={downloading}
					onClick={onDownload}
				>
					{downloading ? (
						<Spinner className="size-4" />
					) : (
						<ArrowDownToLine className="size-4" />
					)}
					下载模型
				</Button>
			</div>
		</section>
	);
}

type PermissionsSetupProps = {
	accessibilityStatus?: string;
	microphoneStatus?: string;
	hotkeyParts: string[];
	onPermissionAction: (
		permission: OSPermission,
		status: string | undefined,
	) => void;
	permissionsReady: boolean;
	canFinish: boolean;
	handleFinish: () => void;
};

function PermissionsSetup({
	accessibilityStatus,
	microphoneStatus,
	hotkeyParts,
	onPermissionAction,
	permissionsReady,
	canFinish,
	handleFinish,
}: PermissionsSetupProps) {
	const statusMap: Record<OSPermission, string | undefined> = {
		microphone: microphoneStatus,
		accessibility: accessibilityStatus,
	};

	return (
		<div className="flex flex-col gap-6">
			<Card className="p-4">
				<CardContent className="px-0 space-y-5">
					{PERMISSIONS.map((permission) => {
						const status = statusMap[permission.key];
						const ready = status === "granted" || status === "notNeeded";
						return (
							<div
								key={permission.key}
								className="flex items-center justify-between"
							>
								<div className="flex items-center gap-3 text-foreground">
									<div className="rounded-full bg-primary/10 p-2 text-primary">
										<permission.icon className="size-5" />
									</div>
									<div className="flex flex-col gap-1">
										<p className="text-sm font-medium">{permission.title}</p>
										<p className="text-xs text-muted-foreground">
											{permission.description}
										</p>
									</div>
								</div>
								<Button
									disabled={ready}
									onClick={() => onPermissionAction(permission.key, status)}
									size="sm"
									className="rounded-full"
									variant={ready ? "outline" : "default"}
								>
									{ready ? "已完成" : permission.action}
								</Button>
							</div>
						);
					})}
				</CardContent>
			</Card>

			<section className="rounded-xl border p-4">
				<div className="flex flex-col gap-4">
					<div className="flex flex-col gap-1">
						<p className="text-base font-semibold text-foreground">
							试着说一句话
						</p>
						<p className="text-xs text-muted-foreground">
							按{" "}
							<KbdGroup className="gap-2">
								{hotkeyParts.map((part, index) => (
									<span key={`${part}`} className="flex items-center gap-2">
										<Kbd>{part}</Kbd>
										{index < hotkeyParts.length - 1 && (
											<span className="text-sm text-muted-foreground">+</span>
										)}
									</span>
								))}
							</KbdGroup>{" "}
							开始录音，再按一次即可粘贴到下方输入框。
						</p>
					</div>
					<Textarea
						className="h-32 resize-none"
						placeholder={
							permissionsReady
								? "按快捷键试试：例如“记得 3 点提醒我发送周报”。"
								: "完成上方权限授权后即可在这里试试语音输入。"
						}
						disabled={!permissionsReady}
					/>
				</div>
			</section>
			<div className="flex items-center justify-center">
				<Button
					size="lg"
					className="rounded-full"
					disabled={!canFinish}
					onClick={handleFinish}
				>
					{canFinish ? "开始使用妙语" : "完成上述步骤后即可开始"}
				</Button>
			</div>
		</div>
	);
}

function formatBytes(bytes: number) {
	if (!bytes) {
		return "0 MB";
	}
	return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

const SPECIAL_KEY_LABELS: Record<string, string> = {
	Space: "Space",
	Enter: "Enter",
	Escape: "Esc",
	Backspace: "⌫",
	Tab: "Tab",
	ArrowUp: "↑",
	ArrowDown: "↓",
	ArrowLeft: "←",
	ArrowRight: "→",
};

function formatHotkeyParts(hotkey?: Hotkey | null) {
	if (!hotkey) {
		return ["Option", "Space"];
	}
	const parts: string[] = [];
	if (hotkey.meta) parts.push("Command");
	if (hotkey.ctrl) parts.push("Control");
	if (hotkey.alt) parts.push("Option");
	if (hotkey.shift) parts.push("Shift");

	let code = hotkey.code;
	if (code.startsWith("Key")) {
		code = code.slice(3).toUpperCase();
	} else if (code.startsWith("Digit")) {
		code = code.slice(5);
	} else {
		code = SPECIAL_KEY_LABELS[code] ?? code;
	}
	parts.push(code);
	return parts;
}
