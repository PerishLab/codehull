import { cli, flags } from "@/lib/cli.ts";
import { bin } from "@/lib/std/cmd.ts";
import { io } from "@/lib/std/io.ts";

const REG = "git.perish.top/perishlab";
const CREDS = `${Deno.env.get("HOME")}/.cargo/credentials.toml`;

function usage(): void {
  io.print("Usage: runseal :ship");
  io.print("");
  io.print("Build and push the codehull api+web images and helm chart");
  io.print("to the perish registry, one version train. From a clean main.");
}

const args = cli.parse(Deno.args, { boolean: ["help", "h"] });
if (flags(args).help()) {
  flags(args).positionals("ship", { allowHelp: true });
  usage();
  Deno.exit(0);
}
flags(args).positionals("ship");

const root = await bin("git").text(["rev-parse", "--show-toplevel"]);
const branch = await bin("git").text(["branch", "--show-current"]);
if (branch !== "main") {
  io.fail(`ship: publish from main, not ${branch}`);
}
const dirty = await bin("git").text(["status", "--short"]);
if (dirty.trim() !== "") {
  io.fail("ship: working tree must be clean");
}

const version = await current(root);
io.print(`==> ship v${version}`);

await forge("api", "deploy/api.Dockerfile", true, root, version);
await forge("web", "deploy/web.Dockerfile", false, root, version);
await chart(root, version);

io.print("ship: clean");

async function forge(
  face: string,
  file: string,
  needsCargo: boolean,
  root: string,
  version: string,
): Promise<void> {
  const image = `${REG}/codehull-${face}:${version}`;
  if (await present(image)) {
    io.print(`==> codehull-${face} v${version} already pushed`);
    return;
  }
  io.print(`==> build codehull-${face}`);
  const build = ["build", "--network=host", "-f", file, "-t", image];
  if (needsCargo) {
    build.push("--secret", `id=cargo,src=${CREDS}`);
  }
  build.push(".");
  await bin("docker").run(build, {
    cwd: root,
    env: { DOCKER_BUILDKIT: "1" },
  });
  io.print(`==> push codehull-${face}`);
  await bin("docker").run(["push", image], { cwd: root });
}

async function chart(root: string, version: string): Promise<void> {
  const image = `oci://${REG}/charts/codehull`;
  if (
    await bin("helm").status(["show", "chart", image, "--version", version], {
      stdout: "null",
      stderr: "null",
    }) === 0
  ) {
    io.print(`==> codehull chart v${version} already pushed`);
    return;
  }
  io.print("==> package chart");
  await bin("helm").run([
    "package",
    "charts/codehull",
    "--version",
    version,
    "--app-version",
    version,
    "--destination",
    `${root}/.local`,
  ], { cwd: root });
  io.print("==> push chart");
  await bin("helm").run([
    "push",
    `${root}/.local/codehull-${version}.tgz`,
    `oci://${REG}/charts`,
  ], { cwd: root });
}

async function present(image: string): Promise<boolean> {
  return await bin("docker").status(["manifest", "inspect", image], {
    stdout: "null",
    stderr: "null",
  }) === 0;
}

async function current(root: string): Promise<string> {
  const cargo = await Deno.readTextFile(`${root}/Cargo.toml`);
  const chart = await Deno.readTextFile(`${root}/charts/codehull/Chart.yaml`);
  const cargoVersion = cargo.match(/^version = "([^"]+)"$/m)?.[1];
  const chartVersion = chart.match(/^version: ([^\s]+)$/m)?.[1];
  const appVersion = chart.match(/^appVersion: "?([^"\s]+)"?$/m)?.[1];
  if (cargoVersion === undefined) {
    io.fail("ship: no workspace version in Cargo.toml");
  }
  if (chartVersion !== cargoVersion || appVersion !== cargoVersion) {
    io.fail("ship: Cargo and chart versions must match");
  }
  return cargoVersion!;
}
