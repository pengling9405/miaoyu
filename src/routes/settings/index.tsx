import ComputerDesktopIcon from "@heroicons/react/24/outline/ComputerDesktopIcon";
import MoonIcon from "@heroicons/react/24/outline/MoonIcon";
import SunIcon from "@heroicons/react/24/outline/SunIcon";
import { createFileRoute } from "@tanstack/react-router";
import { getVersion } from "@tauri-apps/api/app";
import { check as checkUpdater, type Update } from "@tauri-apps/plugin-updater";
import { useCallback, useEffect, useId, useState } from "react";
import { Switch } from "~/components/animate/switch";
import {
	ToggleGroup,
	ToggleGroupItem,
} from "~/components/animate-ui/components/radix/toggle-group";
import { HotkeySetting } from "~/components/settings/hotkey";
import { SettingRow } from "~/components/settings/row";
import { useTheme } from "~/components/theme/provider";
import { Input } from "~/components/ui/input";
import { Label } from "~/components/ui/label";
import { Textarea } from "~/components/ui/textarea";
import type {
	AppTheme,
	AudioFlowPanelPosition,
	Hotkey,
	HotkeyAction,
} from "~/lib/tauri";
import { commands, events } from "~/lib/tauri";
import { hotkeysStore, settingsStore } from "~/store";

type ThemeValue = AppTheme;

const THEME_DEFAULT: ThemeValue = "system";
const AUDIO_FLOW_PANEL_POSITION_DEFAULT: AudioFlowPanelPosition =
	"bottomCenter";

// 默认系统提示词
const DEFAULT_SYSTEM_PROMPT = `你是一个专业的文字润色助手。请对用户提供的语音识别文本进行智能优化：
1. 修正语音识别可能出现的错误
2. 添加合适的标点符号
3. 优化语句使其更加通顺自然
4. 保持原意不变，不要添加或删除关键信息
5. 直接返回优化后的文本，不要添加任何解释或前缀`;

export const Route = createFileRoute("/settings/")({
	component: RouteComponent,
});

function RouteComponent() {
	const settingsQuery = settingsStore.useQuery();
	const hotkeysQuery = hotkeysStore.useQuery();
	const { setTheme } = useTheme();
	const theme = settingsQuery.data?.theme ?? THEME_DEFAULT;
	const audioFlowPanelPosition =
		settingsQuery.data?.audioFlowPanelPosition ??
		AUDIO_FLOW_PANEL_POSITION_DEFAULT;

	// Generate unique IDs for form fields
	const apiKeyFieldId = useId();
	const systemPromptFieldId = useId();

	// 开机启动
	const [autostartEnabled, setAutostartEnabled] = useState(false);
	const [isLoadingAutostart, setIsLoadingAutostart] = useState(true);
	const [isCheckingUpdate, setIsCheckingUpdate] = useState(false);
	const [isInstallingUpdate, setIsInstallingUpdate] = useState(false);

	// LLM 配置
	const [llmApiKey, setLlmApiKey] = useState(
		settingsQuery.data?.llmApiKey ?? "",
	);
	const [llmSystemPrompt, setLlmSystemPrompt] = useState(
		settingsQuery.data?.llmSystemPrompt ?? DEFAULT_SYSTEM_PROMPT,
	);
	const [appVersion, setAppVersion] = useState<string>("");
	const [updateMessage, setUpdateMessage] = useState<string | null>(null);
	const [availableUpdate, setAvailableUpdate] = useState<Update | null>(null);

	// 同步从 store 读取的值
	useEffect(() => {
		if (settingsQuery.data?.llmApiKey)
			setLlmApiKey(settingsQuery.data.llmApiKey);
		// 系统提示词：使用 store 中的值，如果没有则使用默认值
		setLlmSystemPrompt(
			settingsQuery.data?.llmSystemPrompt ?? DEFAULT_SYSTEM_PROMPT,
		);
	}, [settingsQuery.data]);

	const checkForUpdates = useCallback(async (announceResult: boolean) => {
		setIsCheckingUpdate(true);
		try {
			const update = await checkUpdater();

			if (update) {
				setAvailableUpdate(update);
				setUpdateMessage(`发现新版本 ${update.version}`);
			} else {
				setAvailableUpdate(null);
				if (announceResult) {
					setUpdateMessage("当前已经是最新版本");
				} else {
					setUpdateMessage(null);
				}
			}
		} catch (error) {
			console.error("Failed to check updates:", error);
			if (announceResult) {
				setUpdateMessage("检查更新失败，请稍后重试");
			}
		} finally {
			setIsCheckingUpdate(false);
		}
	}, []);

	useEffect(() => {
		void (async () => {
			try {
				const version = await getVersion();
				setAppVersion(version);
			} catch (error) {
				console.error("Failed to load app version:", error);
			}
		})();

		void checkForUpdates(false);
	}, [checkForUpdates]);

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

	const handleAutostartChange = async (checked: boolean) => {
		try {
			await commands.setAutostartEnabled(checked);
			setAutostartEnabled(checked);
		} catch (error) {
			console.error("Failed to set autostart:", error);
		}
	};

	const handleInstallUpdate = async () => {
		if (!availableUpdate) return;

		setIsInstallingUpdate(true);
		try {
			await availableUpdate.downloadAndInstall();
		} catch (error) {
			console.error("Failed to install update:", error);
			setUpdateMessage("更新安装失败，请稍后重试");
		} finally {
			setIsInstallingUpdate(false);
		}
	};

	const updateAudioFlowPanelPosition = (
		value: AudioFlowPanelPosition | undefined,
	) => {
		if (!value || value === audioFlowPanelPosition) return;
		void settingsQuery.set({ audioFlowPanelPosition: value });
		void events.audioFlowPanelPositionChanged
			.emit({ position: value })
			.catch(() => {});
	};

	const updateLlmConfig = async () => {
		await settingsQuery.set({
			llmApiKey: llmApiKey || null,
			llmSystemPrompt: llmSystemPrompt || null,
		});
	};
	return (
		<div className="bg-background text-foreground min-h-screen">
			<div className="mx-auto flex w-full max-w-2xl flex-col gap-6 p-4">
				<header className="space-y-1">
					<h1 className="text-lg font-semibold">设置</h1>
					<p className="text-sm text-muted-foreground">
						管理妙语在桌面端设置选项。
					</p>
				</header>

				{/* 通用 */}
				<section className="space-y-2.5">
					<h2 className="text-base font-medium">通用</h2>
					<div className="rounded-2xl border border-border bg-card shadow-sm px-2.5">
						<SettingRow
							title="开机自启动"
							description="系统启动时自动运行应用。"
							action={
								<Switch
									checked={autostartEnabled}
									onCheckedChange={handleAutostartChange}
									disabled={isLoadingAutostart}
								/>
							}
						/>
					</div>
				</section>

				{/* 版本与更新 */}
				<section className="space-y-2">
					<h2 className="text-base font-medium">版本与更新</h2>
					<div className="rounded-2xl border border-border bg-card shadow-sm p-3">
						<div className="flex flex-wrap items-center justify-between gap-3">
							<div className="flex flex-col">
								<p className="text-sm font-medium">当前版本</p>
								<p className="text-sm text-muted-foreground">
									{appVersion || "--"}
								</p>
							</div>
							<div className="flex items-center gap-3">
								<p className="text-xs text-muted-foreground min-w-[160px] text-right">
									{updateMessage ?? ""}
								</p>
								<button
									onClick={() => void checkForUpdates(true)}
									disabled={isCheckingUpdate}
									className="px-2.5 py-1.5 text-sm rounded-lg border border-border hover:bg-muted transition-colors disabled:opacity-60 disabled:cursor-not-allowed"
								>
									{isCheckingUpdate ? "正在检查..." : "检查更新"}
								</button>
								{availableUpdate && (
									<button
										onClick={() => void handleInstallUpdate()}
										disabled={isInstallingUpdate}
										className="px-2.5 py-1.5 text-sm rounded-lg bg-primary text-primary-foreground hover:bg-primary/90 transition-colors disabled:opacity-60 disabled:cursor-not-allowed"
									>
										{isInstallingUpdate ? "正在更新..." : "立即更新"}
									</button>
								)}
							</div>
						</div>
					</div>
				</section>

				{/* 外观主题 */}
				<section className="space-y-2.5">
					<h2 className="text-base font-medium">外观</h2>
					<div className="rounded-2xl border border-border bg-card shadow-sm px-2.5">
						<SettingRow
							title="主题"
							description="选择应用的外观主题。"
							action={
								<div className="flex flex-col items-end gap-2">
									<ToggleGroup
										size="sm"
										type="single"
										variant="outline"
										value={theme}
										onValueChange={(value) =>
											value && setTheme(value as ThemeValue)
										}
									>
										<ToggleGroupItem
											size="icon"
											value="system"
											aria-label="自动跟随系统"
										>
											<ComputerDesktopIcon />
										</ToggleGroupItem>
										<ToggleGroupItem
											size="icon"
											value="light"
											aria-label="浅色主题"
										>
											<SunIcon />
										</ToggleGroupItem>
										<ToggleGroupItem
											size="icon"
											value="dark"
											aria-label="深色主题"
										>
											<MoonIcon />
										</ToggleGroupItem>
									</ToggleGroup>
								</div>
							}
						/>
					</div>
				</section>

				{/* 快捷键 */}
				<section className="space-y-2">
					<h2 className="text-base font-medium">快捷键</h2>
					<div className="rounded-2xl border border-border bg-card shadow-sm px-2.5">
						<SettingRow
							title="语音识别快捷键"
							description="按下快捷键开始语音识别。"
							action={
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
							}
						/>
					</div>
				</section>

				{/* 语音悬浮窗 */}
				<section className="space-y-2">
					<h2 className="text-base font-medium">语音悬浮窗</h2>
					<div className="rounded-2xl border border-border bg-card shadow-sm px-2.5">
						<SettingRow
							title="停靠位置"
							description="语音浮窗在桌面停靠位置。"
							action={
								<div className="flex flex-col items-end gap-2">
									<ToggleGroup
										size="sm"
										type="single"
										value={audioFlowPanelPosition}
										variant="outline"
										onValueChange={(value) =>
											updateAudioFlowPanelPosition(
												value as AudioFlowPanelPosition | undefined,
											)
										}
									>
										<ToggleGroupItem value="topCenter" aria-label="顶部居中">
											<svg
												width="100%"
												height="100%"
												viewBox="0 0 24 24"
												fill="none"
												xmlns="http://www.w3.org/2000/svg"
											>
												<path
													d="M17.5 6.5L6.5 6.5M7.8 3H16.2C17.8802 3 18.7202 3 19.362 3.32698C19.9265 3.6146 20.3854 4.07354 20.673 4.63803C21 5.27976 21 6.11984 21 7.8V16.2C21 17.8802 21 18.7202 20.673 19.362C20.3854 19.9265 19.9265 20.3854 19.362 20.673C18.7202 21 17.8802 21 16.2 21H7.8C6.11984 21 5.27976 21 4.63803 20.673C4.07354 20.3854 3.6146 19.9265 3.32698 19.362C3 18.7202 3 17.8802 3 16.2V7.8C3 6.11984 3 5.27976 3.32698 4.63803C3.6146 4.07354 4.07354 3.6146 4.63803 3.32698C5.27976 3 6.11984 3 7.8 3Z"
													stroke="currentColor"
													strokeWidth="2"
													strokeLinecap="round"
													strokeLinejoin="round"
												/>
											</svg>
											<span>顶部居中</span>
										</ToggleGroupItem>
										<ToggleGroupItem value="bottomCenter" aria-label="底部居中">
											<svg
												width="100%"
												height="100%"
												viewBox="0 0 24 24"
												fill="none"
												xmlns="http://www.w3.org/2000/svg"
											>
												<path
													d="M17.5 17.6L6.5 17.6M7.8 3H16.2C17.8802 3 18.7202 3 19.362 3.32698C19.9265 3.6146 20.3854 4.07354 20.673 4.63803C21 5.27976 21 6.11984 21 7.8V16.2C21 17.8802 21 18.7202 20.673 19.362C20.3854 19.9265 19.9265 20.3854 19.362 20.673C18.7202 21 17.8802 21 16.2 21H7.8C6.11984 21 5.27976 21 4.63803 20.673C4.07354 20.3854 3.6146 19.9265 3.32698 19.362C3 18.7202 3 17.8802 3 16.2V7.8C3 6.11984 3 5.27976 3.32698 4.63803C3.6146 4.07354 4.07354 3.6146 4.63803 3.32698C5.27976 3 6.11984 3 7.8 3Z"
													strokeWidth="2"
													stroke="currentColor"
													strokeLinecap="round"
													strokeLinejoin="round"
												/>
											</svg>
											<span>底部居中</span>
										</ToggleGroupItem>
									</ToggleGroup>
								</div>
							}
						/>
					</div>
				</section>

				{/* 大语言模型 */}
				<section className="space-y-2">
					<h2 className="text-base font-medium">大语言模型</h2>
					<div className="rounded-2xl border border-border bg-card shadow-sm p-2.5">
						<div className="space-y-1 mb-4">
							<h3 className="text-sm font-medium">DeepSeek</h3>
							<p className="text-sm text-muted-foreground">
								对识别后的文字进行智能润色和优化。
							</p>
						</div>
						<div className="space-y-2">
							<div className="space-y-1.5">
								<Label htmlFor={apiKeyFieldId} className="text-sm">
									API Key
								</Label>
								<Input
									id={apiKeyFieldId}
									name="api_key"
									type="password"
									value={llmApiKey}
									onChange={(e) => setLlmApiKey(e.target.value)}
									placeholder="Deep Seek API Key"
									className="h-9"
								/>
							</div>
							<div className="space-y-1.5">
								<div className="flex items-center justify-between">
									<Label htmlFor={systemPromptFieldId} className="text-sm">
										系统提示词
									</Label>
									<button
										type="button"
										onClick={() => setLlmSystemPrompt(DEFAULT_SYSTEM_PROMPT)}
										className="text-xs text-muted-foreground hover:text-foreground transition-colors"
									>
										重置为默认
									</button>
								</div>
								<Textarea
									id={systemPromptFieldId}
									name="system_prompt"
									value={llmSystemPrompt}
									onChange={(e) => setLlmSystemPrompt(e.target.value)}
									placeholder="输入系统提示词，用于指导 AI 如何处理识别后的文字"
									rows={4}
								/>
							</div>
							<div className="flex justify-end pt-2">
								<button
									onClick={updateLlmConfig}
									className="px-2.5 py-1.5 text-sm bg-primary text-primary-foreground rounded-lg hover:bg-primary/90 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
								>
									保存
								</button>
							</div>
						</div>
					</div>
				</section>
			</div>
		</div>
	);
}
