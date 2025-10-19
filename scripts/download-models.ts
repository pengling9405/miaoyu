import { mkdir, rm, access } from "node:fs/promises";
import path from "node:path";
import { tmpdir } from "node:os";
import { fileURLToPath } from "node:url";
import { randomUUID } from "node:crypto";
import { execFile } from "node:child_process";
import { promisify } from "node:util";

const execFileAsync = promisify(execFile);

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
    files: ["model.int8.onnx", "tokens.txt"],
  },
  {
    key: "punc",
    url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/punctuation-models/sherpa-onnx-punct-ct-transformer-zh-en-vocab272727-2024-04-12.tar.bz2",
    archive: true,
    files: ["model.onnx", "tokens.json"],
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
  console.log(`⬇️  下载 ${url}`);
  const response = await fetch(url);
  if (!response.ok || !response.body) {
    throw new Error(`下载失败：${url}（HTTP ${response.status}）`);
  }

  await ensureDir(path.dirname(destination));
  await Bun.write(destination, response);
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

async function cleanupModelDir(directory: string) {
  await rm(path.join(directory, ".git"), { recursive: true, force: true });
  await rm(path.join(directory, ".gitignore"), { force: true });
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
      await cleanupModelDir(destDir);
    } finally {
      await rm(tempArchive, { force: true });
    }
  } else {
    const filename = task.files[0];
    const target = path.join(destDir, filename);
    await downloadFile(task.url, target);
    await cleanupModelDir(destDir);
  }

  console.log(`✅ ${task.key} 模型准备完成。`);
}

async function main() {
  console.log("🧰 准备下载妙语离线语音模型…");
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
