import { createFileRoute } from "@tanstack/react-router";
import {
	Keyboard,
	Mic,
	Notebook,
	Palette,
	PersonStanding,
	Power,
} from "lucide-react";
import { useEffect, useState } from "react";
import { useCheckPermissions, useOpenPermissionSettings } from "~/hooks/use-permissions";
import { Switch } from "~/components/animate/switch";
import { Dashboard } from "~/components/layouts/dashboard";
import { HotkeySetting } from "~/components/settings/hotkey";
import { SettingRow } from "~/components/settings/row";
import { ThemeToggle } from "~/components/settings/theme-toggle";
import { Badge } from "~/components/ui/badge";
import { Button } from "~/components/ui/button";
import { Card } from "~/components/ui/card";
import { DEFAULT_DIARY_HOTKEY } from "~/constants/hotkeys";
import type {
	Hotkey,
	HotkeyAction,
	OSPermission,
	OSPermissionStatus,
} from "~/lib/tauri";
import { commands } from "~/lib/tauri";
import { hotkeysStore } from "~/store";

export const Route = createFileRoute("/(dashboard)/settings")({
	component: RouteComponent,
});

function RouteComponent() {
	const hotkeysQuery = hotkeysStore.useQuery();
	const permissionsQuery = useCheckPermissions(true);
	const openPermissionSettings = useOpenPermissionSettings();

	// 开机启动
	const [autostartEnabled, setAutostartEnabled] = useState(false);
	const [isLoadingAutostart, setIsLoadingAutostart] = useState(true);

	// 加载开机启动状态
	useEffect(() => {
		const loadAutostartStatus = async () => {
			try {
				const enabled = await commands.getAutostartEnabled();
				setAutostartEnabled(enabled);
			} catch (error) {
				console.error("Failed to load autostart status:", error);
			} finally {
				setIsLoadingAutostart(false);
			}
		};
		void loadAutostartStatus();
	}, []);

	useEffect(() => {
		const id = window.setInterval(() => {
			void permissionsQuery.refetch();
		}, 2000);
		return () => window.clearInterval(id);
	}, [permissionsQuery.refetch]);

	const handleAutostartChange = async (checked: boolean) => {
		try {
			await commands.setAutostartEnabled(checked);
			setAutostartEnabled(checked);
		} catch (error) {
			console.error("Failed to set autostart:", error);
		}
	};

	const handleOpenPermissionSettings = async (permission: OSPermission) => {
		try {
			await openPermissionSettings.mutateAsync(permission);
		} catch (error) {
			console.error("Failed to open permission settings:", error);
		}
	};

	const generalRows = [
		{
			id: "autostart",
			title: "登录时启动",
			description: "系统登录后自动运行妙语。",
			icon: <Power className="size-4" />,
			action: (
				<Switch
					checked={autostartEnabled}
					onCheckedChange={handleAutostartChange}
					disabled={isLoadingAutostart}
				/>
			),
		},
		{
			id: "theme",
			title: "应用主题外观",
			description: "浅色、深色或跟随系统自动切换。",
			icon: <Palette className="size-4" />,
			action: <ThemeToggle />,
		},
	];

	const permissionRows = [
		{
			id: "microphone",
			title: "麦克风权限",
			description: "用于录音转写，系统弹窗同意后即可。",
			icon: <Mic className="size-4" />,
			status: permissionsQuery.data?.microphone,
			permission: "microphone" as const,
		},
		{
			id: "accessibility",
			title: "辅助功能权限",
			description: "首次安装需在“隐私与安全性 › 辅助功能”手动勾选“妙语”，用于自动粘贴与快捷键捕捉。",
			icon: <PersonStanding className="size-4" />,
			status: permissionsQuery.data?.accessibility,
			permission: "accessibility" as const,
		},
	];

	const renderPermissionBadge = (status?: OSPermissionStatus) => {
		switch (status) {
			case "granted":
				return <Badge>已授权</Badge>;
			case "notNeeded":
				return <Badge variant="secondary">系统无需</Badge>;
			case "empty":
				return <Badge variant="secondary">待授权</Badge>;
			case "denied":
				return <Badge variant="destructive">未授权</Badge>;
			default:
				return <Badge variant="secondary">检查中…</Badge>;
		}
	};

	const hotkeyRows = [
		{
			id: "dictation",
			title: "语音识别",
			description: "按下快捷键即可开始语音识别。",
			icon: <Keyboard className="size-4" />,
			action: (
				<HotkeySetting
					currentHotkey={hotkeysQuery.data?.hotkeys?.startDictating}
					onUpdate={async (hotkey) => {
						const nextHotkeys: Partial<Record<HotkeyAction, Hotkey>> = {
							...(hotkeysQuery.data?.hotkeys ?? {}),
						};

						if (hotkey) {
							nextHotkeys.startDictating = hotkey;
						} else {
							delete nextHotkeys.startDictating;
						}

						await hotkeysQuery.set({
							hotkeys: nextHotkeys,
						});

						await commands.setHotkey("startDictating", hotkey);
					}}
				/>
			),
		},
		{
			id: "diary",
			title: "语音日记",
			description: "为语音日记指定快捷键，随时记录灵感。",
			icon: <Notebook className="size-4" />,
			action: (
				<HotkeySetting
					currentHotkey={hotkeysQuery.data?.hotkeys?.startVoiceDiary}
					defaultHotkey={DEFAULT_DIARY_HOTKEY}
					onUpdate={async (hotkey) => {
						const nextHotkeys: Partial<Record<HotkeyAction, Hotkey>> = {
							...(hotkeysQuery.data?.hotkeys ?? {}),
						};

						if (hotkey) {
							nextHotkeys.startVoiceDiary = hotkey;
						} else {
							delete nextHotkeys.startVoiceDiary;
						}

						await hotkeysQuery.set({
							hotkeys: nextHotkeys,
						});

						await commands.setHotkey("startVoiceDiary", hotkey);
					}}
				/>
			),
		},
	];

	return (
		<Dashboard>
			<div className="flex flex-col">
				<div className="py-4 flex flex-col gap-1.5">
					<h3 className="text-lg font-bold">设置</h3>
					<p className="text-muted-foreground text-xs">根据您的偏好配置妙语</p>
				</div>
				<div className="flex flex-col gap-8">
					<section className="space-y-2.5">
						<h2 className="text-base font-medium">常规</h2>
						<Card className="px-4 py-0 gap-0">
							<ul className="divide-y divide-border py-4">
								{generalRows.map((row) => (
									<li key={row.id} className="py-4 first:pt-0 last:pb-0">
										<SettingRow
											title={row.title}
											description={row.description}
											action={row.action}
											icon={row.icon}
										/>
									</li>
								))}
							</ul>
						</Card>
					</section>

					<section className="space-y-2.5">
						<h2 className="text-base font-medium">权限</h2>
						<Card className="px-4 py-0 gap-0">
							<ul className="divide-y divide-border py-4">
								{permissionRows.map((row) => (
									<li key={row.id} className="py-4 first:pt-0 last:pb-0">
										<SettingRow
											title={row.title}
											description={row.description}
											icon={row.icon}
											action={
												<div className="flex items-center gap-2">
													{renderPermissionBadge(row.status)}
													<Button
														variant="outline"
														size="sm"
														disabled={openPermissionSettings.isPending}
														onClick={() =>
															void handleOpenPermissionSettings(row.permission)
														}
													>
														打开系统设置
													</Button>
													<Button
														variant="ghost"
														size="sm"
														disabled={permissionsQuery.isFetching}
														onClick={() => void permissionsQuery.refetch()}
													>
														刷新状态
													</Button>
												</div>
											}
										/>
									</li>
								))}
							</ul>
						</Card>
					</section>

					<section className="space-y-2">
						<h2 className="text-base font-medium">快捷键</h2>
						<Card className="px-4 py-0 gap-0">
							<ul className="divide-y divide-border py-4">
								{hotkeyRows.map((row) => (
									<li key={row.id} className="py-4 first:pt-0 last:pb-0">
										<SettingRow
											title={row.title}
											description={row.description}
											action={row.action}
											icon={row.icon}
										/>
									</li>
								))}
							</ul>
						</Card>
					</section>
				</div>
			</div>
		</Dashboard>
	);
}
