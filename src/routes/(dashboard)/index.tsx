import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { createFileRoute, Link } from "@tanstack/react-router";
import {
  ArrowUpRight,
  CalendarDays,
  Clock4,
  LayoutGrid,
  Mic,
  Notebook,
  Play,
  StopCircle,
  Trash2,
} from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { CopyButton } from "~/components/copy-button";
import { Dashboard } from "~/components/layouts/dashboard";
import {
  Tabs,
  TabsContent,
  TabsContents,
  TabsList,
  TabsTrigger,
} from "~/components/radix/tabs";
import { Badge } from "~/components/ui/badge";
import { Button } from "~/components/ui/button";
import { Card, CardContent, CardFooter } from "~/components/ui/card";
import { Kbd, KbdGroup } from "~/components/ui/kbd";
import { ScrollArea } from "~/components/ui/scroll-area";
import {
  DEFAULT_DIARY_HOTKEY,
  DEFAULT_DICTATION_HOTKEY,
} from "~/constants/hotkeys";
import {
  type AsrModelStore,
  commands,
  type HistoryEntry,
  type Hotkey,
  type LlmModelStore,
} from "~/lib/tauri";
import { hotkeysStore } from "~/store";

const historyTabs = [
  {
    id: "all",
    label: "全部",
    icon: LayoutGrid,
  },
  {
    id: "dictation",
    label: "听写",
    icon: Mic,
  },
  {
    id: "diary",
    label: "日记",
    icon: Notebook,
  },
] as const;
type HistoryTabId = (typeof historyTabs)[number]["id"];
type HistoryKindFilter = Exclude<HistoryTabId, "all">;
export const Route = createFileRoute("/(dashboard)/")({
  component: RouteComponent,
});
function formatDuration(seconds: number) {
  if (seconds < 60) {
    return `${seconds} 秒`;
  }
  const minutes = Math.floor(seconds / 60);
  const remaining = seconds % 60;
  if (remaining === 0) {
    return `${minutes} 分钟`;
  }
  return `${minutes} 分 ${remaining} 秒`;
}
function formatRelativeTime(isoString: string) {
  const date = new Date(isoString);
  if (Number.isNaN(date.getTime())) {
    return isoString;
  }
  const diff = date.getTime() - Date.now();
  const formatter = new Intl.RelativeTimeFormat("zh-CN", {
    numeric: "auto",
  });
  const minutes = Math.round(diff / (1000 * 60));
  const hours = Math.round(diff / (1000 * 60 * 60));
  const days = Math.round(diff / (1000 * 60 * 60 * 24));
  if (Math.abs(minutes) < 60) {
    return formatter.format(minutes, "minute");
  }
  if (Math.abs(hours) < 24) {
    return formatter.format(hours, "hour");
  }
  return formatter.format(days, "day");
}
function formatTotalDuration(totalSeconds: number) {
  if (totalSeconds <= 0) {
    return "0 秒";
  }
  if (totalSeconds < 60) {
    return `${totalSeconds} 秒`;
  }
  if (totalSeconds < 3600) {
    return `${(totalSeconds / 60).toFixed(1)} 分钟`;
  }
  return `${(totalSeconds / 3600).toFixed(1)} 小时`;
}

const SPECIAL_KEY_LABELS: Record<string, string> = {
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

function hotkeyToParts(hotkey?: Hotkey | null): string[] {
  if (!hotkey) {
    return [];
  }

  const parts: string[] = [];
  if (hotkey.meta) parts.push("Command");
  if (hotkey.ctrl) parts.push("Control");
  if (hotkey.alt) parts.push("Option");
  if (hotkey.shift) parts.push("Shift");

  if (hotkey.code.startsWith("Key")) {
    parts.push(hotkey.code.slice(3));
  } else if (hotkey.code.startsWith("Digit")) {
    parts.push(hotkey.code.slice(5));
  } else {
    parts.push(SPECIAL_KEY_LABELS[hotkey.code] ?? hotkey.code);
  }

  return parts;
}
function RouteComponent() {
  const queryClient = useQueryClient();
  const [activeTab, setActiveTab] = useState<HistoryTabId>("all");
  const [playingId, setPlayingId] = useState<string | null>(null);
  const [deletingId, setDeletingId] = useState<string | null>(null);
  const audioRef = useRef<HTMLAudioElement | null>(null);
  const audioUrlRef = useRef<string | null>(null);
  const supportedModelsQuery = useQuery({
    queryKey: ["supported-models"],
    queryFn: () => commands.getSupportedModels(),
  });
  const historyQuery = useQuery({
    queryKey: ["history", activeTab],
    queryFn: () =>
      commands.listHistoryEntries(
        activeTab === "all"
          ? undefined
          : { kind: activeTab as HistoryKindFilter },
      ),
  });
  const historyStatsQuery = useQuery({
    queryKey: ["history-stats"],
    queryFn: () => commands.getHistoryStats(),
  });
  const modelsStoreQuery = useQuery({
    queryKey: ["models-store"],
    queryFn: () => commands.getModelsStore(),
  });
  const offlineStatusQuery = useQuery({
    queryKey: ["offline-model-status"],
    queryFn: () => commands.getOfflineModelsStatus(),
  });
  const hotkeysQuery = hotkeysStore.useQuery();
  const deleteHistoryMutation = useMutation({
    mutationFn: (id: string) => commands.deleteHistoryEntry(id),
  });
  const historyList = historyQuery.data ?? [];
  const stats = historyStatsQuery.data;
  const modelsStore = modelsStoreQuery.data;
  const offlineStatus = offlineStatusQuery.data;
  const summaryBadges = [
    {
      icon: "🔥",
      label: `${stats?.totalEntries ?? 0} 记录`,
    },
    {
      icon: "🚀",
      label: `${stats?.totalWords ?? 0} 字`,
    },
    {
      icon: "👍",
      label: formatTotalDuration(stats?.totalDurationSeconds ?? 0),
    },
  ];
  const stopPlayback = useCallback(() => {
    const current = audioRef.current;
    if (current) {
      current.pause();
      current.src = "";
      audioRef.current = null;
    }
    if (audioUrlRef.current) {
      URL.revokeObjectURL(audioUrlRef.current);
      audioUrlRef.current = null;
    }
    setPlayingId(null);
  }, []);
  useEffect(() => {
    return () => {
      stopPlayback();
    };
  }, [stopPlayback]);
  useEffect(() => {
    if (playingId && !historyList.some((entry) => entry.id === playingId)) {
      stopPlayback();
    }
  }, [historyList, playingId, stopPlayback]);
  const handlePlayRecord = useCallback(
    async (record: HistoryEntry) => {
      if (!record.audioFilePath) {
        await commands.showNotification(
          "当前记录未保存可播放的音频文件",
          "info",
          null,
        );
        return;
      }
      if (playingId === record.id) {
        stopPlayback();
        return;
      }
      stopPlayback();
      try {
        const base64 = await commands.loadHistoryAudio(record.audioFilePath);
        const binary = Uint8Array.from(atob(base64), (char) =>
          char.charCodeAt(0),
        );
        const blob = new Blob([binary.buffer], { type: "audio/wav" });
        const url = URL.createObjectURL(blob);
        audioUrlRef.current = url;
        const audio = new Audio(url);
        audioRef.current = audio;
        audio.onended = () => stopPlayback();
        audio.onerror = () => stopPlayback();
        setPlayingId(record.id);
        await audio.play();
      } catch (error) {
        stopPlayback();
        const message =
          error instanceof Error ? error.message : "无法播放该音频";
        await commands.showNotification(`播放失败: ${message}`, "error", null);
      }
    },
    [playingId, stopPlayback],
  );
  const handleCopySuccess = useCallback(async () => {
    await commands.showNotification("内容已复制到系统剪贴板", "info", null);
  }, []);
  const handleCopyError = useCallback(async (error: Error) => {
    const message = error.message || "无法写入剪贴板";
    await commands.showNotification(`复制失败: ${message}`, "error", null);
  }, []);
  const removeRecordFromCaches = useCallback(
    (recordId: string, kind: HistoryKindFilter) => {
      const tabsToUpdate: HistoryTabId[] = ["all", kind];
      tabsToUpdate.forEach((tab) => {
        queryClient.setQueryData<HistoryEntry[]>(
          ["history", tab],
          (prev) => prev?.filter((entry) => entry.id !== recordId) ?? prev,
        );
      });
    },
    [queryClient],
  );
  const handleDeleteRecord = useCallback(
    async (record: HistoryEntry) => {
      setDeletingId(record.id);
      try {
        await deleteHistoryMutation.mutateAsync(record.id);
        removeRecordFromCaches(record.id, record.kind as HistoryKindFilter);
        await queryClient.invalidateQueries({ queryKey: ["history"] });
        await queryClient.invalidateQueries({ queryKey: ["history-stats"] });
        if (playingId === record.id) {
          stopPlayback();
        }
        await commands.showNotification("历史记录已删除", "info", null);
      } catch (error) {
        const message =
          error instanceof Error ? error.message : "删除失败，请重试";
        await commands.showNotification(`删除失败: ${message}`, "error", null);
      } finally {
        setDeletingId((prev) => (prev === record.id ? null : prev));
      }
    },
    [
      deleteHistoryMutation,
      playingId,
      queryClient,
      removeRecordFromCaches,
      stopPlayback,
    ],
  );
  const supportedModels = supportedModelsQuery.data;
  const activeLlmModelId =
    modelsStore?.activeLlmModel ?? supportedModels?.llmModels?.[0]?.id;
  const activeLlmEntry =
    modelsStore?.llmModels?.find(
      (entry) => entry.textModelId === activeLlmModelId && entry.active,
    ) ??
    modelsStore?.llmModels?.find(
      (entry) => entry.textModelId === activeLlmModelId,
    );
  const tokenLimit = 5000;
  const textUsage = {
    requests: activeLlmEntry?.freeTotalRequests ?? 0,
    tokens: activeLlmEntry?.freeTotalTokenUsage ?? 0,
  };

  const formattedTokenUsage = new Intl.NumberFormat("zh-CN").format(
    textUsage.tokens,
  );
  const formattedTokenLimit = new Intl.NumberFormat("zh-CN").format(tokenLimit);
  const needsTextNotice = Boolean(
    activeLlmEntry && !activeLlmEntry.apiKey?.trim(),
  );
  const showUsageBanner = needsTextNotice;
  const dictationHotkey =
    hotkeysQuery.data?.hotkeys?.startDictating ?? DEFAULT_DICTATION_HOTKEY;
  const diaryHotkey =
    hotkeysQuery.data?.hotkeys?.startVoiceDiary ?? DEFAULT_DIARY_HOTKEY;
  const renderHotkeyDisplay = useCallback((hotkey?: Hotkey | null) => {
    const parts = hotkeyToParts(hotkey);
    if (parts.length === 0) {
      return (
        <Link
          to="/settings"
          className="rounded-md bg-muted px-2 py-1 text-[11px] text-muted-foreground underline-offset-4 hover:underline"
        >
          去设置快捷键
        </Link>
      );
    }
    const occurrences: Record<string, number> = {};
    return (
      <KbdGroup>
        {parts.map((part, index) => {
          const nextCount = (occurrences[part] ?? 0) + 1;
          occurrences[part] = nextCount;
          return (
            <span
              key={`${part}-${nextCount}`}
              className="flex items-center gap-1"
            >
              {index > 0 && (
                <span className="text-[11px] text-muted-foreground">+</span>
              )}
              <Kbd>{part}</Kbd>
            </span>
          );
        })}
      </KbdGroup>
    );
  }, []);
  const renderHistoryCards = () => {
    if (historyQuery.isLoading) {
      return (
        <div className="rounded-2xl border border-dashed border-border/60 bg-background/40 p-8 text-center">
          <p className="text-sm font-medium">历史记录加载中...</p>
        </div>
      );
    }
    if (historyList.length === 0) {
      const hints: Array<{
        id: string;
        text: string;
        hotkey: Hotkey | null;
      }> = [];
      if (activeTab === "all" || activeTab === "dictation") {
        hints.push({
          id: "dictation",
          text: "使用快捷键快速开始听写",
          hotkey: dictationHotkey,
        });
      }
      if (activeTab === "all" || activeTab === "diary") {
        hints.push({
          id: "diary",
          text: "使用快捷键记录语音日记",
          hotkey: diaryHotkey,
        });
      }

      return (
        <div className="rounded-2xl border border-dashed border-border/60 bg-background/40 p-8 text-center">
          <p className="text-sm font-medium">还没有相关记录</p>
          <div className="mt-3 flex flex-col items-center gap-2 text-xs text-muted-foreground">
            {hints.map((hint) => (
              <div
                key={hint.id}
                className="flex flex-wrap items-center justify-center gap-2"
              >
                <span>{hint.text}</span>
                {renderHotkeyDisplay(hint.hotkey)}
              </div>
            ))}
            {hints.length === 0 && (
              <span>完成一次听写或日记后，内容会出现在这里。</span>
            )}
          </div>
        </div>
      );
    }
    return historyList.map((record) => {
      const isPlaying = playingId === record.id;
      const isDeleting = deletingId === record.id;
      const hasAudio = Boolean(record.audioFilePath);
      return (
        <Card key={record.id} className="group gap-4 py-4 transition">
          <CardContent className="px-4 text-base leading-relaxed text-foreground line-clamp-3 whitespace-pre-line">
            {record.title ?? record.text}
          </CardContent>
          <CardFooter className="flex flex-wrap items-center justify-between gap-2 px-4">
            <div className="flex flex-wrap gap-2 text-xs text-muted-foreground">
              <Badge
                variant="secondary"
                className="gap-1 bg-muted text-foreground"
              >
                <CalendarDays className="size-3" />
                {formatRelativeTime(record.createdAt)}
              </Badge>
              <Badge
                variant="secondary"
                className="gap-1 bg-muted text-foreground"
              >
                <Clock4 className="size-3" />
                {formatDuration(record.durationSeconds)}
              </Badge>
            </div>
            <div className="hidden items-center gap-4 text-xs text-muted-foreground group-hover:flex">
              <Button
                variant="text"
                size="xs"
                className={`flex items-center text-xs gap-1 hover:text-foreground ${!hasAudio ? "opacity-60 cursor-not-allowed" : ""}`}
                disabled={!hasAudio || isDeleting}
                onClick={(event) => {
                  event.preventDefault();
                  event.stopPropagation();
                  void handlePlayRecord(record);
                }}
              >
                {isPlaying ? (
                  <StopCircle className="size-3" />
                ) : (
                  <Play className="size-3" />
                )}
                {hasAudio ? (isPlaying ? "停止" : "播放") : "无音频"}
              </Button>
              <CopyButton
                value={record.text ?? ""}
                tooltip="复制到粘贴板"
                disabled={!record.text || isDeleting}
                onClick={(event) => {
                  event.preventDefault();
                  event.stopPropagation();
                }}
                onCopySuccess={handleCopySuccess}
                onCopyError={handleCopyError}
              />
              <Button
                variant="text"
                size="xs"
                className="text-xs gap-1 text-destructive/60 hover:text-destructive"
                disabled={isDeleting}
                onClick={(event) => {
                  event.preventDefault();
                  event.stopPropagation();
                  void handleDeleteRecord(record);
                }}
              >
                <Trash2 className="size-3" />
                {isDeleting ? "删除中..." : "删除"}
              </Button>
            </div>
          </CardFooter>
        </Card>
      );
    });
  };
  return (
    <Dashboard>
      <div className="flex h-full flex-col gap-8 overflow-hidden py-6">
        <section className="flex flex-wrap items-center justify-between gap-6">
          <p className="text-3xl font-semibold tracking-tight">欢迎回来 👋</p>
          <div className="flex flex-wrap items-center gap-3">
            <div className="flex items-center gap-3 rounded-lg bg-accent px-5 py-2 text-sm">
              {summaryBadges.map((badge) => (
                <span key={badge.label} className="flex items-center gap-1">
                  <span>{badge.icon}</span>
                  <span className="text-foreground">{badge.label}</span>
                </span>
              ))}
            </div>
          </div>
        </section>
        {showUsageBanner && (
          <section className="py-3">
            <div className="flex flex-col gap-4 rounded-2xl border border-border/50 bg-card/50 p-5 text-sm shadow-sm">
              <div className="flex flex-wrap items-start justify-between gap-3">
                <div className="flex flex-col gap-1">
                  <p className="text-base font-semibold text-foreground">
                    体验版额度
                  </p>
                  <p className="text-xs text-muted-foreground dark:text-white/60">
                    配置文本生成模型的 API 密钥即可解除体验版限制。
                  </p>
                </div>
                <div className="flex flex-wrap items-center gap-3 text-foreground">
                  <div className="flex flex-col gap-1.5">
                    <div className="flex gap-4">
                      <span className="text-xs font-medium text-muted-foreground">
                        今日 Token 消耗
                      </span>
                    </div>
                    <div className="flex items-baseline gap-2 text-2xl font-semibold">
                      <span>{formattedTokenUsage}</span>
                      <span className="text-muted-foreground">
                        / {formattedTokenLimit}
                      </span>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </section>
        )}
        <section className="flex min-h-0 flex-1 flex-col">
          <Tabs
            value={activeTab}
            onValueChange={(value) => setActiveTab(value as HistoryTabId)}
            className="flex h-full flex-col space-y-4"
          >
            <div className="flex flex-wrap items-end justify-between gap-4">
              <div className="flex flex-col gap-1">
                <h2 className="text-xl font-semibold">历史记录</h2>
                <p className="text-xs text-muted-foreground">
                  查看存储在本设备上的转录历史记录
                </p>
              </div>
              <TabsList>
                {historyTabs.map((tab) => (
                  <TabsTrigger key={tab.id} value={tab.id} className="gap-1.5">
                    <tab.icon className="size-3" />
                    {tab.label}
                  </TabsTrigger>
                ))}
              </TabsList>
            </div>
            <ScrollArea className="h-full w-full pr-4">
              <TabsContents className="mb-15">
                <TabsContent
                  value={activeTab}
                  className="flex h-full min-h-0 flex-col"
                >
                  <div className="flex flex-col gap-4">
                    {renderHistoryCards()}
                  </div>
                </TabsContent>
              </TabsContents>
            </ScrollArea>
          </Tabs>
        </section>
      </div>
    </Dashboard>
  );
}
