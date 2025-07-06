import { rootNodeFromAnchor } from "@codama/nodes-from-anchor";
import { renderVisitor as renderJs } from "@codama/renderers-js";
import { renderVisitor as renderRust } from "@codama/renderers-rust";
import { execSync } from "child_process";
import {
    bytesValueNode,
    createFromRoot,
    enumValueNode,
    getAllAccounts,
    getAllDefinedTypes,
    setAccountDiscriminatorFromFieldVisitor,
    setStructDefaultValuesVisitor,
    updateProgramsVisitor,
} from "codama";
import { existsSync, mkdirSync, readFileSync } from "fs";

console.log("🚀 Starting code generation...");

// Ensure target/idl directory exists
if (!existsSync("target/idl")) {
  mkdirSync("target/idl", { recursive: true });
}

// Generate IDL using shank
console.log("📝 Generating IDL with shank...");
try {
  execSync("shank idl -o target/idl -r solana-program", { stdio: "inherit" });
} catch (error) {
  console.error("❌ Failed to generate IDL with shank:", error.message);
  process.exit(1);
}

// Read the generated IDL
console.log("📖 Reading generated IDL...");
let idl;
try {
  idl = JSON.parse(
    readFileSync("target/idl/xorca_staking_program.json", "utf8"),
  );
} catch (error) {
  console.error("❌ Failed to read IDL file:", error.message);
  process.exit(1);
}

const node = rootNodeFromAnchor(idl);

// Visitors
const updateProgramNameVisitor = updateProgramsVisitor({
  xorca_staking_program: {
    name: "xorca_staking_program",
  },
});

const discriminators = (account) => [
  account.name,
  {
    field: "discriminator",
    value: enumValueNode("AccountDiscriminator", account.name),
  },
];
const addDiscriminatorVisitor = setAccountDiscriminatorFromFieldVisitor(
  Object.fromEntries(getAllAccounts(node).map(discriminators)),
);

const typePadding = (node) => {
  return [
    node.name,
    Object.fromEntries(
      node.type.fields
        ?.filter((field) => field.name.startsWith("padding"))
        .map((field) => [
          field.name,
          bytesValueNode("utf8", "\0".repeat(field.type.size)),
        ]) ?? [],
    ),
  ];
};
const accountPadding = (node) => {
  return [
    node.name,
    Object.fromEntries(
      node.data.fields
        ?.filter((field) => field.name.startsWith("padding"))
        .map((field) => [
          field.name,
          bytesValueNode("utf8", "\0".repeat(field.type.size)),
        ]) ?? [],
    ),
  ];
};

const addPaddingVisitor = setStructDefaultValuesVisitor({
  ...Object.fromEntries(getAllDefinedTypes(node).map(typePadding)),
  ...Object.fromEntries(getAllAccounts(node).map(accountPadding)),
});

// Ensure output directories exist
const jsOutputDir = "js-client/src/generated";
const rustOutputDir = "rust-client/src/generated";

if (!existsSync(jsOutputDir)) {
  mkdirSync(jsOutputDir, { recursive: true });
}
if (!existsSync(rustOutputDir)) {
  mkdirSync(rustOutputDir, { recursive: true });
}

// Renderers
const jsRenderer = renderJs(jsOutputDir);
const rustRenderer = renderRust(rustOutputDir);

// Generate Codama Clients
console.log("🔧 Generating TypeScript client...");
const codama = createFromRoot(node);
codama.update(updateProgramNameVisitor);
codama.update(addDiscriminatorVisitor);
codama.update(addPaddingVisitor);

try {
  codama.accept(jsRenderer);
  console.log("✅ TypeScript client generated successfully");
} catch (error) {
  console.error("❌ Failed to generate TypeScript client:", error.message);
  process.exit(1);
}

try {
  codama.accept(rustRenderer);
  console.log("✅ Rust client generated successfully");
} catch (error) {
  console.error("❌ Failed to generate Rust client:", error.message);
  process.exit(1);
}

// Formatting
console.log("🎨 Formatting generated code...");
try {
  execSync("yarn prettier './js-client/**/*.{js,jsx,ts,tsx,json}' --write", { stdio: "inherit" });
  execSync("cargo fmt -p xorca", { stdio: "inherit" });
  console.log("✅ Code formatting completed");
} catch (error) {
  console.warn("⚠️  Code formatting failed:", error.message);
}

console.log("🎉 Code generation completed successfully!");
