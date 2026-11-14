#!/usr/bin/env node
// Turns ../openapi.json (emitted by the rust server) into src/api.d.ts.
// Only components + operations are emitted: enough to type the client, small
// enough to review in a diff.
import { readFile, writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const here = dirname(fileURLToPath(import.meta.url));
const specPath = join(here, "..", "openapi.json");
const outPath = join(here, "..", "src", "api.d.ts");

const spec = JSON.parse(await readFile(specPath, "utf8"));

const scalars = new Map([
  ["string", "string"],
  ["integer", "number"],
  ["number", "number"],
  ["boolean", "boolean"],
]);

function tsType(schema, indent = "  ") {
  if (!schema) return "unknown";
  if (schema.$ref) return schema.$ref.split("/").pop().replaceAll(".", "_");
  if (schema.enum) return schema.enum.map((v) => JSON.stringify(v)).join(" | ");
  if (schema.type === "array") return `${tsType(schema.items, indent)}[]`;
  if (schema.oneOf || schema.anyOf) {
    return (schema.oneOf ?? schema.anyOf).map((s) => tsType(s, indent)).join(" | ");
  }
  if (schema.type === "object" || schema.properties) {
    const required = new Set(schema.required ?? []);
    const entries = Object.entries(schema.properties ?? {}).map(([key, value]) => {
      const optional = value.nullable || !required.has(key) ? "?" : "";
      return `${indent}  ${key}${optional}: ${tsType(value, indent + "  ")};`;
    });
    return entries.length ? `{\n${entries.join("\n")}\n${indent}}` : "Record<string, unknown>";
  }
  return scalars.get(schema.type) ?? "unknown";
}

const components = spec.components?.schemas ?? {};
const blocks = [`// Generated from openapi.json by packages/contracts/scripts/generate.mjs.`, ``];

for (const [name, schema] of Object.entries(components)) {
  blocks.push(`export type ${name} = ${tsType(schema)};`, ``);
}

const operations = [];
for (const [path, item] of Object.entries(spec.paths ?? {})) {
  for (const [method, op] of Object.entries(item)) {
    const ok = op.responses?.["200"]?.content?.["application/json"]?.schema;
    operations.push(`  ${JSON.stringify(`${method.toUpperCase()} ${path}`)}: ${tsType(ok)};`);
  }
}
blocks.push(`export type Operations = {\n${operations.join("\n")}\n};`, ``);

await writeFile(outPath, blocks.join("\n"));
console.log(`wrote ${outPath}`);
