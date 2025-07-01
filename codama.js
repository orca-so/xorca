import {
  createFromRoot,
  setAccountDiscriminatorFromFieldVisitor,
  setStructDefaultValuesVisitor,
  enumValueNode,
  getAllAccounts,
  updateProgramsVisitor,
  getAllDefinedTypes,
  bytesValueNode,
} from "codama";
import { renderVisitor as renderJs } from "@codama/renderers-js";
import { renderVisitor as renderRust } from "@codama/renderers-rust";
import { rootNodeFromAnchor } from "@codama/nodes-from-anchor";
import { readFileSync } from "fs";
import { execSync } from "child_process";

execSync("shank idl -o target/idl -r solana-program");

const idl = JSON.parse(
  readFileSync("target/idl/xorca_staking_program.json", "utf8"),
);

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

// Renderers
const jsRenderer = renderJs("js-client/src/generated");
const rustRenderer = renderRust("rust-client/src/generated");

// Generate Codama Clients
const codama = createFromRoot(node);
codama.update(updateProgramNameVisitor);
codama.update(addDiscriminatorVisitor);
codama.update(addPaddingVisitor);
codama.accept(jsRenderer);
codama.accept(rustRenderer);

// Formatting
execSync("yarn prettier './js-client/**/*.{js,jsx,ts,tsx,json}' --write");
execSync("cargo fmt -p xorca");
