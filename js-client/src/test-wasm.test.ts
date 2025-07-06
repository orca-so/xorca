import { existsSync, readFileSync } from "fs";
import { join } from "path";
import { describe, expect, it } from "vitest";

describe("WASM Infrastructure", () => {
  describe("Generated Files", () => {
    it("should have correct WASM file structure", () => {
      const wasmDir = join(__dirname, "generated", "wasm");
      const packageJsonPath = join(wasmDir, "package.json");

      if (existsSync(packageJsonPath)) {
        const packageJson = JSON.parse(readFileSync(packageJsonPath, "utf8"));

        // Check that it's a valid WASM package
        expect(packageJson.name).toBe("xorca");
        expect(packageJson.type).toBe("module");
        expect(packageJson.main).toBe("xorca.js");
      }
    });

    it("should have TypeScript definitions", () => {
      const wasmDir = join(__dirname, "generated", "wasm");
      const dtsPath = join(wasmDir, "xorca.d.ts");

      if (existsSync(dtsPath)) {
        const dtsContent = readFileSync(dtsPath, "utf8");

        // Check that math functions are exported
        expect(dtsContent).toContain("export function add");
        expect(dtsContent).toContain("export function multiply");
        expect(dtsContent).toContain("export function square");
        expect(dtsContent).toContain("export function power");
      }
    });
  });

  describe("Build Scripts", () => {
    it("should have build-wasm script", () => {
      const buildScriptPath = join(
        __dirname,
        "..",
        "..",
        "scripts",
        "build-wasm.js",
      );
      expect(existsSync(buildScriptPath)).toBe(true);
    });

    it("should have copy-wasm script", () => {
      const copyScriptPath = join(
        __dirname,
        "..",
        "..",
        "scripts",
        "copy-wasm.js",
      );
      expect(existsSync(copyScriptPath)).toBe(true);
    });
  });

  describe("Package Configuration", () => {
    it("should have WASM build script in package.json", () => {
      const packageJsonPath = join(__dirname, "..", "..", "package.json");
      const packageJson = JSON.parse(readFileSync(packageJsonPath, "utf8"));

      expect(packageJson.scripts["build:wasm"]).toBe(
        "node scripts/build-wasm.js",
      );
      expect(packageJson.scripts["copy:wasm"]).toBe(
        "node scripts/copy-wasm.js",
      );
    });
  });

  describe("Rust Configuration", () => {
    it("should have WASM feature in rust-client Cargo.toml", () => {
      const cargoTomlPath = join(
        __dirname,
        "..",
        "..",
        "rust-client",
        "Cargo.toml",
      );
      const cargoTomlContent = readFileSync(cargoTomlPath, "utf8");

      expect(cargoTomlContent).toContain(
        'wasm = ["dep:wasm-bindgen", "dep:js-sys"]',
      );
      expect(cargoTomlContent).toContain(
        'wasm-bindgen = { version = "^0.2", optional = true }',
      );
    });

    it("should have math module in rust-client", () => {
      const mathModulePath = join(
        __dirname,
        "..",
        "..",
        "rust-client",
        "src",
        "math",
        "mod.rs",
      );
      expect(existsSync(mathModulePath)).toBe(true);

      const mathContent = readFileSync(mathModulePath, "utf8");
      expect(mathContent).toContain("#[wasm_bindgen]");
      expect(mathContent).toContain("pub fn add");
      expect(mathContent).toContain("pub fn multiply");
      expect(mathContent).toContain("pub fn square");
      expect(mathContent).toContain("pub fn power");
    });
  });

  describe("Integration Points", () => {
    it("should export WASM from main index", () => {
      const indexPath = join(__dirname, "index.ts");
      const indexContent = readFileSync(indexPath, "utf8");

      expect(indexContent).toContain('export * from "./generated/wasm"');
    });
  });
});
