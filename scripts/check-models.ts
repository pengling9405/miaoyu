import { access } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const REQUIRED_MODELS = [
  {
    path: "asr/model.int8.onnx",
    description: "Paraformer ASR 模型",
  },
  {
    path: "asr/am.mvn",
    description: "ASR 均值方差参数",
  },
  {
    path: "asr/tokens.txt",
    description: "ASR 词表",
  },
  {
    path: "punc/model.onnx",
    description: "标点补全模型",
  },
  {
    path: "punc/tokens.json",
    description: "标点模型词表",
  },
  {
    path: "punc/config.yaml",
    description: "标点模型配置",
  },
  {
    path: "vad/silero_vad.onnx",
    description: "Silero VAD 模型",
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

async function main() {
  const missing: string[] = [];

  for (const model of REQUIRED_MODELS) {
    const fullPath = path.join(MODELS_ROOT, model.path);
    if (!(await exists(fullPath))) {
      missing.push(`${model.description}（${model.path}）`);
    }
  }

  if (missing.length > 0) {
    console.error("❌ 模型文件缺失：");
    for (const item of missing) {
      console.error(`  - ${item}`);
    }
    console.error(
      "\n请运行 `bun run download-models` 或按照 README 中的指引手动放置模型文件。",
    );
    process.exit(1);
  }

  console.log("✅ 所有必需模型文件已准备就绪。");
}

await main();
