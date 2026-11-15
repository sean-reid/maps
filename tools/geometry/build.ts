import { createHash } from "node:crypto";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import mapshaper from "mapshaper";

const SOURCE =
  "https://raw.githubusercontent.com/nvkelso/natural-earth-vector/v5.1.2/geojson/ne_50m_admin_0_countries.geojson";
const SHA256 = "3e458fc036ad0a66411f2c1e6cac49c5d7bfb81cb1123bc513b22511a2b7fdeb";
const KEEP = "20%";

const root = resolve(import.meta.dirname, "../..");
const cache = resolve(root, ".cache");
const cached = resolve(cache, "ne_50m_admin_0_countries.geojson");

const sha256 = (data: Buffer) => createHash("sha256").update(data).digest("hex");
if (!existsSync(cached) || sha256(readFileSync(cached)) !== SHA256) {
  const res = await fetch(SOURCE);
  if (!res.ok) throw new Error(`${SOURCE}: ${res.status}`);
  const body = Buffer.from(await res.arrayBuffer());
  if (sha256(body) !== SHA256) throw new Error(`Natural Earth checksum mismatch: ${sha256(body)}`);
  mkdirSync(cache, { recursive: true });
  writeFileSync(cached, body);
}
const source = readFileSync(cached);

const commands = [
  "-i in.geojson",
  `-filter 'ADM0_A3 !== "ATA"'`,
  `-each 'iso = ISO_A3_EH && ISO_A3_EH !== "-99" ? ISO_A3_EH : ADM0_A3, name = NAME'`,
  "-sort 'ADM0_A3 === iso ? 0 : 1'",
  "-dissolve iso copy-fields=name",
  "-proj eqearth",
  `-simplify weighted ${KEEP} keep-shapes`,
  "-clean",
  "-rename-layers atoms",
  "-o atoms.json format=topojson quantization=200000",
].join(" ");
const out = await mapshaper.applyCommands(commands, { "in.geojson": source });
const produced = out["atoms.json"];
if (!produced) throw new Error("mapshaper wrote no atoms.json");
const text = produced.toString();

interface Topology {
  arcs: number[][][];
  objects: { atoms: { geometries: { properties: { iso: string; name: string } }[] } };
}
const topology = JSON.parse(text) as Topology;
const atoms = topology.objects.atoms.geometries;
for (const { properties } of atoms) {
  if (!/^[A-Z]{3}$/.test(properties.iso) || !properties.name) {
    throw new Error(`bad atom ${JSON.stringify(properties)}`);
  }
}
const points = topology.arcs.reduce((n, arc) => n + arc.length, 0);

writeFileSync(resolve(root, "public/geo/atoms.json"), text);
console.log(
  `${atoms.length} atoms, ${topology.arcs.length} arcs, ${points} points, ${text.length} bytes`,
);
