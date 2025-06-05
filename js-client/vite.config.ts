import { defineConfig } from "vitest/config";
import { resolve } from "path";
import wasm from "vite-plugin-wasm";
import dts from "vite-plugin-dts";
import { viteStaticCopy } from "vite-plugin-static-copy";
import topLevelAwait from "vite-plugin-top-level-await";
import { execSync } from "child_process";
import { rmSync } from "fs";
import { PluginOption } from "vite";

const modes = new Set(["browser", "node", "test"]);

export default defineConfig(({ mode }) => {
  if (!modes.has(mode)) {
    throw new Error(`Invalid mode: ${mode}`);
  }

  const target = mode === "node" ? "esnext" : undefined;

  const plugins: PluginOption[] = [
    {
      name: "wasm-build",
      enforce: "pre",
      buildStart() {
        execSync(
          "yarn wasm-pack build --release --out-dir ../js-client/src/math --out-name index ../math-lib --features wasm",
        );
        rmSync("src/LICENSE");
      },
    },
    wasm(),
    dts(),
    viteStaticCopy({
      targets: [
        {
          src: "src/math/index.d.ts",
          dest: "math",
        },
      ],
    }),
  ];

  if (mode === "browser") {
    plugins.push(topLevelAwait());
  }

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
