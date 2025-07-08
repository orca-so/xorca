import { execSync } from 'child_process';
import { resolve } from 'path';
import { defineConfig, Plugin } from 'vite';
import dts from 'vite-plugin-dts';

const modes = new Set(['browser', 'node', 'test']);

// Plugin to build WASM files before build
function wasmBuildPlugin(): Plugin {
  return {
    name: 'wasm-build-plugin',
    buildStart() {
      console.log('[wasm-build-plugin] Building WASM files...');
      try {
        const wasmOutputDir = resolve(__dirname, 'src/generated/wasm');
        execSync(`wasm-pack build ../rust-client --target web --out-dir ${wasmOutputDir}`, {
          stdio: 'inherit',
          cwd: __dirname,
        });
        console.log('[wasm-build-plugin] WASM build completed successfully');
      } catch (error) {
        console.error('[wasm-build-plugin] WASM build failed:', error);
        throw error;
      }
    },
  };
}

export default defineConfig(({ mode }) => {
  if (!modes.has(mode)) {
    throw new Error(`Invalid mode: ${mode}`);
  }

  const target = mode === 'node' ? 'esnext' : undefined;

  const plugins: Plugin[] = [dts(), wasmBuildPlugin()];

  return {
    build: {
      lib: {
        entry: resolve(__dirname, 'src/index.ts'),
        formats: ['es'],
        fileName: `index.${mode}`,
      },
      target,
      emptyOutDir: false,
    },
    resolve: { alias: { src: resolve('src/') } },
    test: {
      includeSource: ['**/*.{js,ts}'],
    },
    plugins: plugins,
    define: {
      'import.meta.vitest': 'undefined',
    },
  };
});
