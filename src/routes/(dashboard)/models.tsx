import { listen } from "@tauri-apps/api/event";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { createFileRoute } from "@tanstack/react-router";
import {
  ChevronDown,
  Crosshair,
  Download,
  History as HistoryIcon,
  SquareArrowOutUpRight,
  WholeWord,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import { RadioGroup } from "~/components/animate-ui/components/radix/radio-group";
import { DeepSeekIcon } from "~/components/icons/deepseek";
import { QwenIcon } from "~/components/icons/qwen";
import { TextIcon } from "~/components/icons/text";
import { Dashboard } from "~/components/layouts/dashboard";
import { ModelCard } from "~/components/model-card";
import {
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
  TabsContents,
} from "@/components/radix/tabs";
import { Button } from "~/components/ui/button";
import { Input } from "~/components/ui/input";
import { Label } from "~/components/ui/label";
import { Select } from "~/components/ui/select";
import { Progress } from "~/components/ui/progress";
import { commands, type LlmModelStore } from "~/lib/tauri";
import { ASRIcon } from "~/components/icons/asr";
import { PARAFORMER_MODEL_ID } from "~/constants/models";

export const Route = createFileRoute("/(dashboard)/models")({
  component: RouteComponent,
});

const SENSEVOICE_MODEL_ID =
  "sherpa-onnx-sense-voice-zh-en-ja-ko-yue-2024-07-17";

const textModelMeta: Record<string, { icon: JSX.Element; wrapper: string }> = {
  deepseek: {
    icon: <DeepSeekIcon className="size-8 fill-white" />,
    wrapper: "bg-[rgb(77,107,254,1)]",
  },
  qwen: {
    icon: <QwenIcon className="size-8 fill-white" />,
    wrapper: "bg-[rgba(97,92,237,1)]",
  },
};

const asrModelMeta: Record<string, { icon: JSX.Element; wrapper: string }> = {
  [PARAFORMER_MODEL_ID]: {
    icon: <QwenIcon className="size-8 fill-white" />,
    wrapper: "bg-[rgba(97,92,237,1)]",
  },
  [SENSEVOICE_MODEL_ID]: {
    icon: <QwenIcon className="size-8 fill-white" />,
    wrapper: "bg-[rgba(74,46,217,1)]",
  },
};

const FRIENDLY_ERROR_MESSAGES: Record<string, string> = {
  deepseek: "调用 DeepSeek API 失败，API 密钥错误",
  qwen: "调用通义千问 API 失败，API 密钥错误",
};
type ModelTab = "llm" | "asr";

type TextModelFormValue = {
  provider: string;
  apiKey: string;
};

type StatusVariant = "success" | "error";

type StatusMessage = {
  variant: StatusVariant;
  text: string;
};

type StatusState = Record<string, StatusMessage | null>;

type DownloadProgressState = Record<string, number>;

type OfflineDownloadProgressEvent = {
  modelId: string;
  receivedBytes: number;
  totalBytes: number | null;
};

const formatAsrDuration = (hours: number) => {
  if (!hours) {
    return "0 秒";
  }
  const totalSeconds = Math.max(0, Math.round(hours * 3600));
  if (totalSeconds < 60) {
    return `${totalSeconds} 秒`;
  }
  const totalMinutes = Math.max(1, Math.round(hours * 60));
  if (totalMinutes < 60) {
    return `${totalMinutes} 分钟`;
  }
  const roundedHours = Math.round(hours * 10) / 10;
  const formatted =
    Number.isInteger(roundedHours) && roundedHours >= 1
      ? `${roundedHours}`
      : roundedHours.toFixed(1);
  return `${formatted} 小时`;
};

function RouteComponent() {
  const queryClient = useQueryClient();
  const supportedModelsQuery = useQuery({
    queryKey: ["supported-models"],
    queryFn: () => commands.getSupportedModels(),
  });
  const modelsStoreQuery = useQuery({
    queryKey: ["models-store"],
    queryFn: () => commands.getModelsStore(),
  });
  const offlineStatusQuery = useQuery({
    queryKey: ["offline-model-status"],
    queryFn: () => commands.getOfflineModelsStatus(),
  });

  const llmModels = supportedModelsQuery.data?.llmModels ?? [];
  const asrModels = supportedModelsQuery.data?.asrModels ?? [];
  const modelsStoreData = modelsStoreQuery.data;
  const offlineStatus = offlineStatusQuery.data;
  const [downloadingModel, setDownloadingModel] = useState<string | null>(null);

  const selectedTextModel = useMemo(() => {
    if (modelsStoreData?.activeLlmModel) {
      return modelsStoreData.activeLlmModel;
    }
    return llmModels[0]?.id ?? "";
  }, [llmModels, modelsStoreData?.activeLlmModel]);

  const selectedAsrModel = useMemo(() => {
    if (modelsStoreData?.activeAsrModel) {
      return modelsStoreData.activeAsrModel;
    }
    return asrModels[0]?.id ?? "";
  }, [asrModels, modelsStoreData?.activeAsrModel]);

  const [formValues, setFormValues] = useState<
    Record<string, TextModelFormValue>
  >({});
  const [statusMessages, setStatusMessages] = useState<StatusState>({});
  const [activeApiForm, setActiveApiForm] = useState<string | null>(null);
  const [savingModel, setSavingModel] = useState<string | null>(null);
  const [downloadProgress, setDownloadProgress] =
    useState<DownloadProgressState>({});

  const getLlmEntries = useCallback(
    (modelId: string) =>
      (modelsStoreData?.llmModels ?? []).filter(
        (entry) => entry.textModelId === modelId,
      ),
    [modelsStoreData?.llmModels],
  );

  const getActiveLlmEntry = useCallback(
    (modelId: string): LlmModelStore | undefined => {
      const entries = getLlmEntries(modelId);
      return entries.find((entry) => entry.active) ?? entries[0];
    },
    [getLlmEntries],
  );

  const getAsrEntries = useCallback(
    (modelId: string) =>
      (modelsStoreData?.asrModels ?? []).filter(
        (entry) => entry.modelId === modelId,
      ),
    [modelsStoreData?.asrModels],
  );

  const getActiveAsrEntry = useCallback(
    (modelId: string) => {
      const entries = getAsrEntries(modelId);
      return entries.find((entry) => entry.active) ?? entries[0];
    },
    [getAsrEntries],
  );

  const asrUsage = useCallback(
    (modelId: string) => {
      const entry = getActiveAsrEntry(modelId);
      return {
        requests: entry?.totalRequests ?? 0,
        hours: entry?.totalHours ?? 0,
      };
    },
    [getActiveAsrEntry],
  );

  const getOfflineModelStatus = useCallback(
    (modelId: string) =>
      offlineStatus?.models?.find((item) => item.id === modelId),
    [offlineStatus],
  );

  const getProviderOptions = useCallback(
    (modelId: string) => {
      return llmModels.find((model) => model.id === modelId)?.providers ?? [];
    },
    [llmModels],
  );

  const getStoredFormValue = useCallback(
    (modelId: string): TextModelFormValue => {
      const entry = getActiveLlmEntry(modelId);
      const providers = getProviderOptions(modelId);
      return {
        provider: entry?.provider ?? providers[0]?.id ?? "",
        apiKey: entry?.apiKey ?? "",
      };
    },
    [getActiveLlmEntry, getProviderOptions],
  );

  useEffect(() => {
    if (llmModels.length === 0) {
      return;
    }
    setFormValues((prev) => {
      const next = { ...prev };
      llmModels.forEach((model) => {
        next[model.id] = getStoredFormValue(model.id);
      });
      return next;
    });
  }, [getStoredFormValue, llmModels]);

  useEffect(() => {
    let unlisten: (() => void) | null = null;
    const setupListener = async () => {
      try {
        unlisten = await listen<OfflineDownloadProgressEvent>(
          "offline-model-download-progress",
          (event) => {
            const { modelId, receivedBytes, totalBytes } = event.payload;
            setDownloadProgress((prev) => {
              const next = { ...prev };
              const percent =
                totalBytes && totalBytes > 0
                  ? Math.min(
                      100,
                      Math.round((receivedBytes / totalBytes) * 100),
                    )
                  : (next[modelId] ?? 0);
              next[modelId] = percent;
              return next;
            });
          },
        );
      } catch (error) {
        console.error(error);
      }
    };
    void setupListener();
    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  }, []);

  const setActiveTextModelMutation = useMutation({
    mutationFn: (modelId: string) => commands.setActiveTextModel(modelId),
    onSuccess: (data) => {
      queryClient.setQueryData(["models-store"], data);
    },
  });

  const setActiveAsrModelMutation = useMutation({
    mutationFn: (modelId: string) => commands.setActiveAsrModel(modelId),
    onSuccess: (data) => {
      queryClient.setQueryData(["models-store"], data);
    },
  });

  const handleModelChange = (value: string) => {
    if (!value) {
      return;
    }
    setActiveTextModelMutation.mutate(value);
  };

  const handleProviderChange = (modelId: string, providerId: string) => {
    setFormValues((prev) => ({
      ...prev,
      [modelId]: {
        ...prev[modelId],
        provider: providerId,
      },
    }));
    clearStatusMessage(modelId);
  };

  const handleToggleTextForm = (modelId: string) => {
    setActiveApiForm((prev) => (prev === modelId ? null : modelId));
    clearStatusMessage(modelId);
  };

  const clearStatusMessage = (modelId: string) => {
    setStatusMessages((prev) => ({
      ...prev,
      [modelId]: null,
    }));
  };

  const handleApiKeyChange = (modelId: string, value: string) => {
    setFormValues((prev) => ({
      ...prev,
      [modelId]: {
        ...prev[modelId],
        apiKey: value,
      },
    }));
    clearStatusMessage(modelId);
  };

  const handleCancel = (modelId: string) => {
    setFormValues((prev) => ({
      ...prev,
      [modelId]: getStoredFormValue(modelId),
    }));
    setActiveApiForm(null);
    clearStatusMessage(modelId);
  };

  const handleTestAndSave = async (modelId: string) => {
    const formValue = formValues[modelId];
    if (!formValue) {
      return;
    }

    setSavingModel(modelId);
    clearStatusMessage(modelId);

    try {
      const providerId = formValue.provider;
      const apiKey = formValue.apiKey.trim();
      await commands.testLlmApiKey(modelId, providerId, apiKey || null);
      const updated = await commands.updateTextModelCredentials(
        modelId,
        providerId,
        apiKey || null,
      );
      queryClient.setQueryData(["models-store"], updated);
      setStatusMessages((prev) => ({
        ...prev,
        [modelId]: {
          variant: "success",
          text: "测试成功，已保存",
        },
      }));
      setActiveApiForm(null);
    } catch (error) {
      console.error(error);
      setStatusMessages((prev) => ({
        ...prev,
        [modelId]: {
          variant: "error",
          text:
            FRIENDLY_ERROR_MESSAGES[modelId] ?? "调用 API 失败，API 密钥错误",
        },
      }));
    } finally {
      setSavingModel((prev) => (prev === modelId ? null : prev));
    }
  };

  const textModelsEmpty = llmModels.length === 0;
  const asrModelsEmpty = asrModels.length === 0;

  const handleDownloadAsrModel = async (modelId: string) => {
    if (downloadingModel) {
      return;
    }
    setDownloadProgress((prev) => ({ ...prev, [modelId]: 0 }));
    setDownloadingModel(modelId);
    try {
      const status = await commands.downloadOfflineModels(modelId);
      queryClient.setQueryData(["offline-model-status"], status);
      const store = await commands.setActiveAsrModel(modelId);
      queryClient.setQueryData(["models-store"], store);
    } catch (error) {
      console.error(error);
    } finally {
      setDownloadingModel(null);
      setDownloadProgress((prev) => {
        const next = { ...prev };
        delete next[modelId];
        return next;
      });
    }
  };

  const handleAsrModelChange = (value: string) => {
    if (!value) {
      return;
    }
    const target = asrModels.find((model) => model.id === value);
    const status = target?.offline
      ? offlineStatus?.models?.find((item) => item.id === value)
      : undefined;
    if (target?.offline && !(status?.ready ?? false)) {
      void commands.showNotification("请先下载离线模型后再使用", "info", null);
      return;
    }
    setActiveAsrModelMutation.mutate(value);
  };

  const [activeTab, setActiveTab] = useState<ModelTab>("llm");

  return (
    <Dashboard>
      <Tabs
        value={activeTab}
        onValueChange={(value) => setActiveTab(value as ModelTab)}
        className="space-y-6"
      >
        <div className="space-y-1">
          <h1 className="text-2xl font-semibold">模型配置</h1>
          <p className="text-sm text-muted-foreground">
            在这里管理文本生成与离线语音识别模型。
          </p>
        </div>
        <TabsList>
          <TabsTrigger value="llm">
            <TextIcon className="size-4" />
            文本生成
          </TabsTrigger>
          <TabsTrigger value="asr">
            <ASRIcon className="size-4" />
            语音识别
          </TabsTrigger>
        </TabsList>
        <TabsContents>
          <TabsContent value="llm">
            {textModelsEmpty ? (
              <div className="rounded-lg border border-dashed p-8 text-center text-sm text-muted-foreground">
                暂无可用文本模型，请检查配置。
              </div>
            ) : (
              <RadioGroup
                value={selectedTextModel}
                onValueChange={handleModelChange}
                className="space-y-4"
              >
                {llmModels.map((model) => {
                  const meta = textModelMeta[model.id] ?? {
                    icon: <TextIcon className="size-6" />,
                    wrapper: "bg-muted",
                  };
                  const formValue = formValues[model.id];
                  const usage = getActiveLlmEntry(model.id);
                  const providers = getProviderOptions(model.id);
                  const providerLink = providers.find(
                    (provider) => provider.id === formValue?.provider,
                  )?.apiKeyUrl;

                  return (
                    <ModelCard
                      key={model.id}
                      id={model.id}
                      value={model.id}
                      icon={meta.icon}
                      iconWrapperClassName={meta.wrapper}
                      title={model.title}
                      description={
                        <div className="flex flex-1 items-center justify-between">
                          <div className="flex gap-3">
                            <div className="flex items-center gap-1">
                              <Crosshair className="size-3" />
                              <p className="text-xs">
                                请求次数：{usage?.totalRequests ?? 0}
                              </p>
                            </div>
                            <div className="flex items-center gap-1">
                              <WholeWord className="size-3" />
                              <p className="text-xs">
                                Token 消耗：{usage?.totalTokenUsage ?? 0}
                              </p>
                            </div>
                          </div>
                          <button
                            type="button"
                            className="text-xs text-muted-foreground transition hover:text-primary flex items-center gap-0.5"
                            onClick={(event) => {
                              event.preventDefault();
                              event.stopPropagation();
                              handleToggleTextForm(model.id);
                            }}
                          >
                            设置 API 密钥
                            <ChevronDown className="size-3" />
                          </button>
                        </div>
                      }
                    >
                      {activeApiForm === model.id && formValue && (
                        <div className="flex w-full flex-col gap-3 px-4 -mt-2 pb-4">
                          <div className="flex w-full flex-wrap items-center gap-3 gap-x-8">
                            <Label className="w-20 shrink-0">提供商</Label>
                            <div className="flex flex-1 flex-wrap items-center gap-3">
                              <Select
                                value={formValue.provider}
                                onChange={(event) =>
                                  handleProviderChange(
                                    model.id,
                                    event.target.value,
                                  )
                                }
                                wrapperClassName="flex-1 min-w-[220px]"
                              >
                                {providers.map((provider) => (
                                  <option key={provider.id} value={provider.id}>
                                    {provider.name}
                                  </option>
                                ))}
                              </Select>
                              {providerLink && (
                                <a
                                  href={providerLink}
                                  target="_blank"
                                  className="text-xs text-muted-foreground flex items-center gap-0.5 whitespace-nowrap"
                                >
                                  获取 API 密钥
                                  <SquareArrowOutUpRight className="size-3" />
                                </a>
                              )}
                            </div>
                          </div>
                          <div className="flex w-full flex-wrap items-center gap-3 gap-x-8">
                            <Label className="w-20 shrink-0">API 密钥</Label>
                            <Input
                              type="text"
                              className="flex-1 min-w-[220px]"
                              value={formValue.apiKey}
                              onChange={(event) =>
                                handleApiKeyChange(model.id, event.target.value)
                              }
                              onFocus={() => clearStatusMessage(model.id)}
                            />
                          </div>
                          <div className="flex items-center justify-end gap-3">
                            {statusMessages[model.id] && (
                              <p
                                className={`text-xs ${
                                  statusMessages[model.id]?.variant === "error"
                                    ? "text-destructive"
                                    : "text-muted-foreground"
                                }`}
                              >
                                {statusMessages[model.id]?.text}
                              </p>
                            )}
                            <Button
                              type="button"
                              size="sm"
                              variant="outline"
                              className="text-muted-foreground hover:text-foreground"
                              onClick={() => handleCancel(model.id)}
                            >
                              取消
                            </Button>
                            <Button
                              type="button"
                              size="sm"
                              disabled={
                                savingModel === model.id ||
                                !formValue.apiKey.trim()
                              }
                              onClick={() => void handleTestAndSave(model.id)}
                            >
                              {savingModel === model.id
                                ? "保存中..."
                                : "测试并保存"}
                            </Button>
                          </div>
                        </div>
                      )}
                    </ModelCard>
                  );
                })}
              </RadioGroup>
            )}
          </TabsContent>
          <TabsContent value="asr" className="space-y-4">
            {asrModelsEmpty ? (
              <div className="rounded-lg border border-dashed p-8 text-center text-sm text-muted-foreground">
                暂无可用语音模型，请检查配置。
              </div>
            ) : (
              <RadioGroup
                value={selectedAsrModel ?? ""}
                onValueChange={handleAsrModelChange}
                className="space-y-4"
              >
                {asrModels.map((model) => {
                  const meta = asrModelMeta[model.id] ?? {
                    icon: <QwenIcon className="size-6 fill-white" />,
                    wrapper: "bg-[rgba(97,92,237,1)]",
                  };
                  const usage = asrUsage(model.id);
                  const offlineModel = model.offline
                    ? getOfflineModelStatus(model.id)
                    : undefined;
                  const offlineReady = model.offline
                    ? (offlineModel?.ready ?? false)
                    : true;
                  const downloading = downloadingModel === model.id;
                  const showDownloadButton = model.offline && !offlineReady;
                  const progressValue = downloadProgress[model.id];

                  return (
                    <ModelCard
                      key={model.id}
                      id={`asr-${model.id}`}
                      value={model.id}
                      icon={meta.icon}
                      iconWrapperClassName={meta.wrapper}
                      title={model.title}
                      radioDisabled={showDownloadButton}
                      description={
                        <div className="flex flex-1 items-center justify-between">
                          <div className="flex gap-3">
                            <div className="flex items-center gap-1">
                              <Crosshair className="size-3" />
                              <p className="text-xs">
                                识别次数：{usage.requests}
                              </p>
                            </div>
                            <div className="flex items-center gap-1">
                              <HistoryIcon className="size-3" />
                              <p className="text-xs">
                                累计时长：{formatAsrDuration(usage.hours)}
                              </p>
                            </div>
                          </div>
                          {showDownloadButton && !downloading && (
                            <Button
                              type="button"
                              size="xs"
                              variant="text"
                              onClick={() =>
                                void handleDownloadAsrModel(model.id)
                              }
                            >
                              <Download className="size-3" />
                              下载模型
                            </Button>
                          )}
                        </div>
                      }
                    >
                      {downloading && (
                        <div className="flex flex-col gap-2 w-full px-4 pb-4 -mt-4 text-xs text-muted-foreground">
                          <div className="flex items-center justify-between">
                            <span>
                              {progressValue !== undefined &&
                              progressValue >= 100
                                ? "解压模型文件中..."
                                : "模型下载中..."}
                            </span>
                            <span className="text-primary">
                              {progressValue !== undefined
                                ? `${Math.min(progressValue, 100)}%`
                                : "准备中..."}
                            </span>
                          </div>
                          <Progress
                            value={Math.min(progressValue ?? 0, 100)}
                            className="animate-pulse"
                            aria-label="模型下载进度"
                          />
                        </div>
                      )}
                    </ModelCard>
                  );
                })}
              </RadioGroup>
            )}
          </TabsContent>
        </TabsContents>
      </Tabs>
    </Dashboard>
  );
}
