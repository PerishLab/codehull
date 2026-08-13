import { bin } from "./cmd.ts";
import { io } from "./io.ts";
import type { Issuer } from "./issuer.ts";

const host = "127.0.0.1";

type Seat = { id: number; head: Record<string, string> };

type Source = { main: string; topic: string; bundle: string };

type Held = { child: Deno.ChildProcess; drain: Promise<void>; log: () => string };

export async function ground(root: string, mint: Issuer, port: number): Promise<void> {
  const dir = `${root}/.local/ground`;
  const base = `http://${host}:${port}/api`;
  await wipe(dir);
  await Deno.mkdir(dir, { recursive: true });
  await Deno.writeTextFile(`${dir}/codehull.toml`, settings(port));
  const seed = await source(dir);
  await bin("cargo").run(["run", "--locked", "-q", "-p", "api", "--", "bootstrap", dir], {
    cwd: root,
    stdout: "null",
  });
  let held = await boot(root, dir, base, mint);
  try {
    const seat = await join(base, mint, "ground");
    const stranger = await join(base, mint, "drifter");
    const repo = await make(base, seat);

    await check("a running process binds metadata to a marked bare seat", async () => {
      const born = await send(base, `/repo/${repo}/git`, seat, "PUT");
      if (born.status !== 200) {
        throw new Error(`provision ${born.status}`);
      }
      const again = await send(base, `/repo/${repo}/git`, seat, "PUT");
      if (again.status !== 200) {
        throw new Error(`provision is not idempotent ${again.status}`);
      }
    });

    await check("exact bundle objects ingest", async () => {
      for (const object of [seed.main, seed.topic]) {
        const res = await send(base, `/repo/${repo}/git/object`, seat, "POST", {
          object,
          bundle: seed.bundle,
        });
        if (res.status !== 200) {
          throw new Error(`ingest ${object} ${res.status}`);
        }
      }
    });

    await check("branch refs advance by compare and swap", async () => {
      await move(base, repo, seat, "refs/heads/main", undefined, seed.main);
      await move(base, repo, seat, "refs/heads/topic", undefined, seed.topic);
      if (await peek(base, repo, seat, "refs/heads/main") !== seed.main) {
        throw new Error("main did not seed");
      }
    });

    await check("stale and absent expectations refuse", async () => {
      const stale = await send(base, `/repo/${repo}/git/ref`, seat, "POST", {
        name: "refs/heads/main",
        before: seed.topic,
        after: seed.topic,
      });
      await stale.body?.cancel();
      if (stale.status !== 409) {
        throw new Error(`stale expectation resolved ${stale.status}`);
      }
      const twice = await send(base, `/repo/${repo}/git/ref`, seat, "POST", {
        name: "refs/heads/main",
        after: seed.topic,
      });
      await twice.body?.cancel();
      if (twice.status !== 409) {
        throw new Error(`expected absence resolved over a live ref ${twice.status}`);
      }
      if (await peek(base, repo, seat, "refs/heads/main") !== seed.main) {
        throw new Error("a refused move still wrote");
      }
    });

    await check("a stranger reaches no seat", async () => {
      for (const path of [`/repo/${repo}/git`, `/repo/${repo}/git/ref?name=refs/heads/main`]) {
        const res = await send(base, path, stranger, "GET");
        await res.body?.cancel();
        if (res.status !== 403) {
          throw new Error(`stranger reached ${path} ${res.status}`);
        }
      }
    });

    await move(base, repo, seat, "refs/heads/main", seed.main, seed.topic);

    await check("both authorities reconstruct after restart", async () => {
      await stop(held);
      held = await boot(root, dir, base, mint);
      const seen = await send(base, `/repo/${repo}/git`, seat, "GET");
      await seen.body?.cancel();
      if (seen.status !== 200) {
        throw new Error(`seat lost across restart ${seen.status}`);
      }
      if (await peek(base, repo, seat, "refs/heads/main") !== seed.topic) {
        throw new Error("main did not survive restart");
      }
      if (await peek(base, repo, seat, "refs/heads/topic") !== seed.topic) {
        throw new Error("topic did not survive restart");
      }
    });
  } catch (err) {
    io.error(held.log());
    throw err;
  } finally {
    await stop(held);
    await wipe(dir);
  }
}

function settings(port: number): string {
  return [
    "[listen]",
    `host = "${host}"`,
    `port = ${port}`,
    'prefix = ""',
    "",
    "[store]",
    'kind = "file"',
    'path = "codehull.sqlite"',
    "",
    "[cache]",
    'kind = "memory"',
    "",
    "[repo]",
    'path = "seats"',
    "",
  ].join("\n");
}

async function boot(root: string, dir: string, base: string, mint: Issuer): Promise<Held> {
  const env: Record<string, string> = {
    API_OIDC_ISSUER: mint.url,
    API_OIDC_AUDIENCE: "codehull",
  };
  const child = new Deno.Command("cargo", {
    args: ["run", "--locked", "-q", "-p", "api", "--", "serve", dir],
    cwd: root,
    env,
    stdin: "null",
    stdout: "null",
    stderr: "piped",
  }).spawn();
  let log = "";
  const drain = (async () => {
    const decoder = new TextDecoder();
    for await (const part of child.stderr) {
      log += decoder.decode(part);
    }
  })();
  const held = { child, drain, log: () => log };
  try {
    await ready(`${base}/health`, 120);
  } catch (err) {
    await stop(held);
    io.error(held.log());
    throw err;
  }
  return held;
}

async function stop(held: Held): Promise<void> {
  try {
    held.child.kill("SIGTERM");
  } catch { /* gone */ }
  try {
    await held.child.status;
  } catch { /* gone */ }
  try {
    await held.drain;
  } catch { /* gone */ }
}

async function source(dir: string): Promise<Source> {
  const seed = `${dir}/seed`;
  await Deno.mkdir(seed, { recursive: true });
  const git = bin("git");
  const at = (args: string[]) => git.text(["-C", seed, ...args], { stderr: "null" });
  await at(["init", "-q", "-b", "main"]);
  await at(["config", "user.name", "Codehull Act"]);
  await at(["config", "user.email", "codehull@example.invalid"]);
  await Deno.writeTextFile(`${seed}/probe`, "main\n");
  await at(["add", "probe"]);
  await at(["commit", "-q", "-m", "seed main"]);
  const main = await at(["rev-parse", "HEAD"]);
  await at(["checkout", "-q", "-b", "topic"]);
  await Deno.writeTextFile(`${seed}/probe`, "topic\n");
  await at(["commit", "-q", "-am", "advance topic"]);
  const topic = await at(["rev-parse", "HEAD"]);
  const pack = `${dir}/seed.bundle`;
  await at(["bundle", "create", pack, "--all"]);
  return { main, topic, bundle: encode(await Deno.readFile(pack)) };
}

function encode(bytes: Uint8Array): string {
  let held = "";
  for (const byte of bytes) {
    held += String.fromCharCode(byte);
  }
  return btoa(held);
}

async function join(base: string, mint: Issuer, sub: string): Promise<Seat> {
  const head = { authorization: `Bearer ${await mint.mint(sub, [])}` };
  const res = await fetch(`${base}/query`, {
    method: "POST",
    headers: { "content-type": "application/json", ...head },
    body: JSON.stringify({ q: `from Actor where sub = "${sub}"` }),
  });
  if (!res.ok) {
    throw new Error(`query ${res.status}`);
  }
  const body = await res.json() as Record<string, unknown>;
  const bags = body.bags as Record<string, unknown> | undefined;
  const rows = bags?.[body.root as string];
  if (!Array.isArray(rows) || rows.length !== 1) {
    throw new Error(`anchor row for ${sub} missing`);
  }
  return { id: (rows[0] as Record<string, number>).id, head };
}

async function make(base: string, seat: Seat): Promise<number> {
  const res = await send(base, "/repo", seat, "POST", {
    name: "truth",
    visibility: "private",
    owner: seat.id,
    trunk: "main",
    archived: false,
  });
  if (res.status !== 201) {
    throw new Error(`POST /repo ${res.status}`);
  }
  return (await res.json() as Record<string, number>).id;
}

async function move(
  base: string,
  repo: number,
  seat: Seat,
  name: string,
  before: string | undefined,
  after: string,
): Promise<void> {
  const body: Record<string, unknown> = { name, after };
  if (before !== undefined) {
    body.before = before;
  }
  const res = await send(base, `/repo/${repo}/git/ref`, seat, "POST", body);
  if (res.status !== 200) {
    await res.body?.cancel();
    throw new Error(`advance ${name} ${res.status}`);
  }
  await res.body?.cancel();
}

async function peek(base: string, repo: number, seat: Seat, name: string): Promise<string | null> {
  const path = `/repo/${repo}/git/ref?name=${encodeURIComponent(name)}`;
  const res = await send(base, path, seat, "GET");
  if (res.status !== 200) {
    await res.body?.cancel();
    throw new Error(`read ${name} ${res.status}`);
  }
  return (await res.json() as Record<string, string | null>).object;
}

async function send(
  base: string,
  path: string,
  seat: Seat,
  verb: string,
  body?: Record<string, unknown>,
): Promise<Response> {
  return await fetch(`${base}${path}`, {
    method: verb,
    headers: { "content-type": "application/json", ...seat.head },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
}

async function ready(url: string, tries: number): Promise<void> {
  for (let i = 0; i < tries; i++) {
    try {
      const res = await fetch(url);
      await res.body?.cancel();
      if (res.ok) {
        return;
      }
    } catch { /* retry */ }
    await sleep(250);
  }
  throw new Error(`timeout ${url}`);
}

async function check(label: string, run: () => Promise<void>): Promise<void> {
  io.print(`==> ${label}`);
  await run();
}

async function wipe(dir: string): Promise<void> {
  try {
    await Deno.remove(dir, { recursive: true });
  } catch { /* absent */ }
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
