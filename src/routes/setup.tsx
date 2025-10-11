import { createFileRoute } from "@tanstack/react-router";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect, useRef } from "react";
import { Button } from "~/components/ui/button";
import {
	useCheckPermissions,
	useOpenPermissionSettings,
	useRequestPermission,
} from "~/hooks/use-permissions";
import { useTimer } from "~/hooks/use-timer";
import type { OSPermission, OSPermissionStatus } from "~/lib/tauri";

const permissions = [
	{
		key: "microphone" as OSPermission,
		title: "麦克风权限",
		description: "妙语需要使用麦克风来录制您的语音并进行识别。",
	},
	{
		key: "accessibility" as OSPermission,
		title: "辅助功能权限",
		description: "妙语需要辅助功能将转写内容自动填入输入框。",
	},
] as const;

export const Route = createFileRoute("/setup")({
	component: RouteComponent,
});

function RouteComponent() {
	const { data: permissionsStatus, refetch } = useCheckPermissions(false);
	const requestPermission = useRequestPermission();
	const openPermissionSettings = useOpenPermissionSettings();
	useTimer({ autoStart: true, tickMs: 250 });
	const hasClosedRef = useRef(false);

	useEffect(() => {
		refetch();
	}, [refetch]);

	useEffect(() => {
		if (!permissionsStatus || hasClosedRef.current) {
			return;
		}
		const microphoneReady =
			permissionsStatus.microphone === "granted" ||
			permissionsStatus.microphone === "notNeeded";
		const accessibilityReady =
			permissionsStatus.accessibility === "granted" ||
			permissionsStatus.accessibility === "notNeeded";

		if (microphoneReady && accessibilityReady) {
			hasClosedRef.current = true;
			const window = getCurrentWindow();
			window.close().catch((error) => {
				console.error("Failed to close setup window:", error);
				hasClosedRef.current = false;
			});
		}
	}, [permissionsStatus]);

	const getButtonText = (status: OSPermissionStatus | undefined) => {
		if (status === "granted") return "已授权";
		if (status === "denied") return "请求权限";
		return "允许";
	};

	const isButtonDisabled = (status: OSPermissionStatus | undefined) => {
		return status === "granted" || status === "notNeeded";
	};

	const handleButtonClick = (
		permission: OSPermission,
		status: OSPermissionStatus | undefined,
	) => {
		if (status === "denied") {
			openPermissionSettings.mutate(permission);
		} else {
			requestPermission.mutate(permission);
		}
	};

	return (
		<div className="flex h-screen items-center justify-center px-5 bg-background">
			<div className="flex flex-col gap-6 w-full">
				<header className="space-y-1 text-center">
					<h1 className="text-xl font-semibold tracking-tight">访问权限授权</h1>
					<p className="text-sm text-muted-foreground">
						妙语需要访问下列权限，请完成授权。
					</p>
				</header>
				<ul className="space-y-4">
					{permissions.map((permission) => {
						const status = permissionsStatus?.[permission.key];
						return (
							<li key={permission.key}>
								<div className="flex items-center justify-between gap-3 sm:flex-row sm:items-center sm:justify-between">
									<div className="space-y-1">
										<p className="text-sm font-medium">{permission.title}</p>
										<p className="text-xs text-muted-foreground">
											{permission.description}
										</p>
									</div>
									<Button
										className="rounded-full"
										size="lg"
										disabled={isButtonDisabled(status)}
										onClick={() => handleButtonClick(permission.key, status)}
									>
										{getButtonText(status)}
									</Button>
								</div>
							</li>
						);
					})}
				</ul>
			</div>
		</div>
	);
}
