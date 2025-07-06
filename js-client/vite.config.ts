import { copyFileSync, existsSync, mkdirSync } from "fs";
import { resolve } from "path";
import { PluginOption } from "vite";
import dts from "vite-plugin-dts";
import { defineConfig } from "vitest/config";

const modes = new Set(["browser", "node", "test"]);

// Plugin to copy WASM files after build
function wasmCopyPlugin(): PluginOption {
  return {
    name: "wasm-copy-plugin",
    closeBundle() {
      const srcDir = resolve(__dirname, "../rust-client/pkg");
      const destDir = resolve(__dirname, "src/generated/wasm");

      // Create the directory if it doesn't exist
      if (!existsSync(destDir)) {
        mkdirSync(destDir, { recursive: true });
      }

      const filesToCopy = ["xorca_bg.wasm", "xorca.js", "xorca.d.ts"];
      filesToCopy.forEach((file) => {
        const src = resolve(srcDir, file);
        const dest = resolve(destDir, file);
        if (existsSync(src)) {
          copyFileSync(src, dest);
          console.log(
            `[wasm-copy-plugin] Copied ${file} to src/generated/wasm/`,
          );
        }
      });
    },
  };
}

export default defineConfig(({ mode }) => {
  if (!modes.has(mode)) {
    throw new Error(`Invalid mode: ${mode}`);
  }

  const target = mode === "node" ? "esnext" : undefined;

  const plugins: PluginOption[] = [dts(), wasmCopyPlugin()];

  return {
    build: {
      lib: {
        entry: resolve(__dirname, "src/index.ts"),
        formats: ["es"],
        fileName: `index.${mode}`,
      },
      target,
      emptyOutDir: false,
    },
    resolve: { alias: { src: resolve("src/") } },
    test: {
      includeSource: ["**/*.{js,ts}"],
    },
    plugins: plugins,
    define: {
      "import.meta.vitest": "undefined",
    },
  };
});
