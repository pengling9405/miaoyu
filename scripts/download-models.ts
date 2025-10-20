import { mkdir, rm, access, readdir } from "node:fs/promises";
import path from "node:path";
import { tmpdir } from "node:os";
import { fileURLToPath } from "node:url";
import { randomUUID } from "node:crypto";
import { execFile } from "node:child_process";
import { promisify } from "node:util";

const execFileAsync = promisify(execFile);

const BUNDLE_VERSION = "2024-03-09";
const BUNDLE_FILENAME = `miaoyu-models-${BUNDLE_VERSION}.tar.bz2`;
const DEFAULT_BUNDLE_URL = `https://github.com/zhanyuilong/miaoyu/releases/download/models-${BUNDLE_VERSION}/${BUNDLE_FILENAME}`;
const BUNDLE_URL =
  process.env.MIAOYU_MODELS_BUNDLE_URL ?? DEFAULT_BUNDLE_URL;

const MODEL_DIRS = ["asr", "punc", "vad"] as const;

type DownloadTask =
  | {
      key: "asr" | "punc";
      url: string;
      archive: true;
      files: string[];
    }
  | {
      key: "vad";
      url: string;
      archive: false;
      files: string[];
    };

const DOWNLOADS: DownloadTask[] = [
  {
    key: "asr",
    url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-paraformer-zh-2024-03-09.tar.bz2",
    archive: true,
    files: ["model.int8.onnx", "tokens.txt", "am.mvn"],
  },
  {
    key: "punc",
    url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/punctuation-models/sherpa-onnx-punct-ct-transformer-zh-en-vocab272727-2024-04-12.tar.bz2",
    archive: true,
    files: ["model.onnx", "tokens.json", "config.yaml"],
  },
  {
    key: "vad",
    url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/silero_vad.onnx",
    archive: false,
    files: ["silero_vad.onnx"],
  },
];

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const MODELS_ROOT = path.resolve(__dirname, "../src-tauri/models");
let curlAvailable: boolean | undefined;

async function exists(filePath: string): Promise<boolean> {
  try {
    await access(filePath);
    return true;
  } catch {
    return false;
  }
}

async function ensureDir(dir: string) {
  await mkdir(dir, { recursive: true });
}

async function downloadFile(url: string, destination: string) {
  if (curlAvailable === undefined) {
    try {
      await execFileAsync("curl", ["--version"]);
      curlAvailable = true;
    } catch {
      curlAvailable = false;
    }
  }
  console.log(`⬇️  下载 ${url}`);
  await ensureDir(path.dirname(destination));
  if (curlAvailable && (process.env.CI || process.env.MIAOYU_USE_CURL === "1")) {
    const args = [
      "-L",
      "--fail",
      "--retry",
      "5",
      "--retry-delay",
      "5",
    ];
    if (process.env.GITHUB_TOKEN && url.startsWith("https://github.com/")) {
      args.push("-H", `Authorization: token ${process.env.GITHUB_TOKEN}`);
      args.push("-H", "Accept: application/octet-stream");
    }
    args.push("-o", destination, url);
    await execFileAsync("curl", args);
  } else {
    const response = await fetch(url);
    if (!response.ok || !response.body) {
      throw new Error(`下载失败：${url}（HTTP ${response.status}）`);
    }
    await Bun.write(destination, response);
  }
}

async function extractArchive(archivePath: string, destination: string) {
  await rm(destination, { recursive: true, force: true });
  await ensureDir(destination);
  await execFileAsync("tar", [
    "-xjf",
    archivePath,
    "-C",
    destination,
    "--strip-components=1",
  ]);
}

async function cleanupModelDir(directory: string, allowed: string[]) {
  await rm(path.join(directory, ".git"), { recursive: true, force: true });
  await rm(path.join(directory, ".gitignore"), { force: true });
  const allowedSet = new Set(
    allowed.map((item) => item.split(path.posix.sep).join(path.sep)),
  );
  await pruneExtraEntries(directory, directory, allowedSet);
}

async function pruneExtraEntries(
  root: string,
  current: string,
  allowed: Set<string>,
) {
  const entries = await readdir(current, { withFileTypes: true });
  for (const entry of entries) {
    const fullPath = path.join(current, entry.name);
    const relative = path.relative(root, fullPath);
    if (entry.isDirectory()) {
      await pruneExtraEntries(root, fullPath, allowed);
      const remaining = await readdir(fullPath);
      if (remaining.length === 0) {
        await rm(fullPath, { recursive: true, force: true });
      }
      continue;
    }
    if (!allowed.has(relative)) {
      await rm(fullPath, { force: true });
    }
  }
}

async function extractBundle(archivePath: string) {
  const baseArgs = ["-xjf", archivePath, "-C", MODELS_ROOT] as const;
  const stripCandidates = [1, 2, 0];

  const hasAllModelDirs = async () => {
    const results = await Promise.all(
      MODEL_DIRS.map((dir) => exists(path.join(MODELS_ROOT, dir))),
    );
    return results.every(Boolean);
  };

  for (const strip of stripCandidates) {
    await rm(MODELS_ROOT, { recursive: true, force: true });
    await ensureDir(MODELS_ROOT);

    const args =
      strip > 0
        ? [...baseArgs, `--strip-components=${strip}`]
        : [...baseArgs];

    try {
      await execFileAsync("tar", args);
    } catch (error) {
      console.warn(
        `⚠️ 解压模型包失败（strip-components=${strip}）：`,
        error instanceof Error ? error.message : error,
      );
      continue;
    }

    if (await hasAllModelDirs()) {
      for (const dir of MODEL_DIRS) {
      await cleanupModelDir(path.join(MODELS_ROOT, dir), DOWNLOADS.find((d) => d.key === dir)?.files ?? []);
      }
      return;
    }
  }

  throw new Error("模型包结构不符合预期，缺少 asr/punc/vad 目录。");
}

async function tryDownloadBundle(): Promise<boolean> {
  const tempArchive = path.join(
    tmpdir(),
    `miaoyu-bundle-${randomUUID()}.tar.bz2`,
  );

  console.log("🎯 尝试从模型发布包下载…");
  try {
    await downloadFile(BUNDLE_URL, tempArchive);
    console.log("📦 解压模型包…");
    await extractBundle(tempArchive);
    console.log("✅ 模型包准备完成。");
    return true;
  } catch (error) {
    console.warn(
      "⚠️ 模型包下载失败，将回退到逐个模型下载。",
      error instanceof Error ? `(${error.message})` : error,
    );
    return false;
  } finally {
    await rm(tempArchive, { force: true });
  }
}

async function processDownload(task: DownloadTask) {
  const destDir = path.join(MODELS_ROOT, task.key);
  const requiredFiles = task.files.map((file) => path.join(destDir, file));
  const alreadyPresent = await Promise.all(
    requiredFiles.map((file) => exists(file)),
  );

  if (alreadyPresent.every(Boolean)) {
    console.log(`✅ ${task.key} 模型已存在，跳过下载。`);
    return;
  }

  if (task.archive) {
    const tempArchive = path.join(
      tmpdir(),
      `miaoyu-${task.key}-${randomUUID()}.tar.bz2`,
    );
    try {
      await downloadFile(task.url, tempArchive);
      console.log(`📦 解压 ${task.key} 模型…`);
      await extractArchive(tempArchive, destDir);
      await cleanupModelDir(destDir, task.files);
    } finally {
      await rm(tempArchive, { force: true });
    }
  } else {
    const filename = task.files[0];
    const target = path.join(destDir, filename);
    await downloadFile(task.url, target);
    await cleanupModelDir(destDir, task.files);
  }

  console.log(`✅ ${task.key} 模型准备完成。`);
}

async function main() {
  console.log("🧰 准备下载妙语离线语音模型…");
  if (await tryDownloadBundle()) {
    return;
  }

  await ensureDir(MODELS_ROOT);

  for (const task of DOWNLOADS) {
    try {
      await processDownload(task);
    } catch (error) {
      console.error(`❌ 处理 ${task.key} 模型失败:`, error);
      process.exitCode = 1;
      return;
    }
  }

  console.log("🎉 所有模型已就绪。");
}

await main();
