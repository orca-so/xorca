import { resolve } from "path";
import { PluginOption } from "vite";
import dts from "vite-plugin-dts";
import { defineConfig } from "vitest/config";

const modes = new Set(["browser", "node", "test"]);

export default defineConfig(({ mode }) => {
  if (!modes.has(mode)) {
    throw new Error(`Invalid mode: ${mode}`);
  }

  const target = mode === "node" ? "esnext" : undefined;

  const plugins: PluginOption[] = [
    dts(),
  ];

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
