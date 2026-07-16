import { cli, flags } from "@/lib/cli.ts";
import { bin } from "@/lib/std/cmd.ts";
import { io } from "@/lib/std/io.ts";

const host = "127.0.0.1";
const port = 13400;
const base = `http://${host}:${port}`;

function usage(): void {
  io.print("Usage: runseal :act");
  io.print("");
  io.print("Forgejo stage acts over the running binary (docs/spec.md).");
}

const args = cli.parse(Deno.args, { boolean: ["help", "h"] });
if (flags(args).help()) {
  flags(args).positionals("act", { allowHelp: true });
  usage();
  Deno.exit(0);
}
flags(args).positionals("act");

const root = await bin("git").text(["rev-parse", "--show-toplevel"]);
const dir = `${root}/.local/act`;
await Deno.mkdir(dir, { recursive: true });
await Deno.writeTextFile(
  `${dir}/keel.toml`,
  `[listen]\nhost = "${host}"\nport = ${port}\nprefix = ""\n\n[store]\nkind = "memory"\n\n[identity]\nunit = "Actor"\n\n[cache]\nkind = "memory"\n`,
);

io.print("==> build forgejo");
await bin("cargo").run(["build", "--locked"], { cwd: root });

io.print(`==> boot forgejo on ${base}`);
const child = new Deno.Command("cargo", {
  args: ["run", "--locked", "--", dir],
  cwd: root,
  stdin: "null",
  stdout: "null",
  stderr: "piped",
}).spawn();

let boot = "";
const drain = (async () => {
  const decoder = new TextDecoder();
  for await (const part of child.stderr) {
    boot += decoder.decode(part);
  }
})();

let failed = false;
try {
  await ready(`${base}/health`, 40);

  io.print("==> act 1: org governance");
  const ada = await join("ada");
  const bob = await join("bob");
  const cy = await join("cy");

  let org = 0;
  await check("owner mints the org subtree", async () => {
    const made = await want("/actor", { login: "lab", kind: "org" }, ada.head);
    org = num(made.id);
    const grab = await post("/actor", { login: "lab2", kind: "org" }, bob.head);
    if (grab.status !== 201) {
      throw new Error(`org create ${grab.status}`);
    }
  });

  let crew = 0;
  await check("team roots under the org via subtree", async () => {
    const made = await want("/team", { name: "owners", mode: "admin", org }, ada.head);
    crew = num(made.id);
    const steal = await post("/team", { name: "grab", mode: "admin", org }, bob.head);
    if (steal.status !== 403) {
      throw new Error(`expected 403, got ${steal.status}`);
    }
  });

  let vault = 0;
  await check("group grant admits members", async () => {
    const made = await want("/repo", {
      name: "vault",
      visibility: "private",
      owner: org,
      trunk: "main",
      archived: false,
    }, ada.head);
    vault = num(made.id);
    await grant(ada.head, `team ${crew}`, "see", "Repo", `row ${vault}`);

    if (seen(bob, vault, await peek(vault, bob.head))) {
      throw new Error("bob sees before joining");
    }
    await want(`/team/${crew}/members`, { right: bob.id }, ada.head);
    if (!(await visible(vault, bob.head))) {
      throw new Error("member cannot see repo");
    }
    if (await visible(vault, cy.head)) {
      throw new Error("stranger sees repo");
    }
  });

  await check("removal revokes access", async () => {
    const pack = await query(
      `from Team where id = "${crew}" link members`,
      ada.head,
    );
    const tie = bond(pack, "team.members")[0];
    const gone = await fetch(
      `${base}/team/${crew}/members/${num(tie.id)}`,
      { method: "DELETE", headers: ada.head },
    );
    if (gone.status !== 204) {
      throw new Error(`remove ${gone.status}`);
    }
    if (await visible(vault, bob.head)) {
      throw new Error("access outlived membership");
    }
  });

  io.print("==> act 2: reactions");
  await check("one reaction per actor-issue-emoji", async () => {
    const pub = await want("/repo", {
      name: "open",
      visibility: "public",
      owner: ada.id,
      trunk: "main",
      archived: false,
    }, ada.head);
    const repo = num(pub.id);
    const issue = await want("/issue", {
      title: "hi",
      body: "",
      closed: false,
      repo,
      author: bob.id,
    }, bob.head);
    const on = num(issue.id);
    await want("/reaction", { emoji: "up", issue: on, actor: bob.id }, bob.head);
    await want("/reaction", { emoji: "tada", issue: on, actor: bob.id }, bob.head);
    const dup = await post(
      "/reaction",
      { emoji: "up", issue: on, actor: bob.id },
      bob.head,
    );
    if (dup.status !== 409) {
      throw new Error(`dup reaction ${dup.status}`);
    }
    const held = await query(
      `from Reaction where issue = "${on}" and emoji = "up"`,
      bob.head,
    );
    const bag = (held.bags as Record<string, Array<Record<string, unknown>>>)
      .reaction;
    const gone = await fetch(`${base}/reaction/${num(bag[0].id)}`, {
      method: "DELETE",
      headers: bob.head,
    });
    if (gone.status !== 204) {
      throw new Error(`undo ${gone.status}`);
    }
    const again = await post(
      "/reaction",
      { emoji: "up", issue: on, actor: bob.id },
      bob.head,
    );
    if (again.status !== 201) {
      throw new Error(`re-react ${again.status}`);
    }
  });

  io.print("act: clean");
} catch (err) {
  failed = true;
  io.error(`act: ${err instanceof Error ? err.message : String(err)}`);
} finally {
  try {
    child.kill("SIGTERM");
  } catch { /* gone */ }
  try {
    await child.status;
  } catch { /* gone */ }
  try {
    await drain;
  } catch { /* gone */ }
  try {
    await Deno.remove(dir, { recursive: true });
  } catch { /* gone */ }
}

if (failed) {
  const _ = boot;
  Deno.exit(1);
}

type Seat = { id: number; head: Record<string, string> };

async function join(login: string): Promise<Seat> {
  const made = await want("/register", { login, kind: "user" });
  const token = made.token as string;
  return { id: num(made.id), head: { authorization: `token ${token}` } };
}

function seen(_seat: Seat, _id: number, status: number): boolean {
  return status === 200;
}

async function visible(id: number, head: Record<string, string>): Promise<boolean> {
  return (await peek(id, head)) === 200;
}

async function peek(id: number, head: Record<string, string>): Promise<number> {
  const res = await fetch(`${base}/repo/${id}`, { headers: head });
  await res.body?.cancel();
  return res.status;
}

async function grant(
  head: Record<string, string>,
  who: string,
  verb: string,
  unit: string,
  scope: string,
): Promise<void> {
  await want("/@grant", { who, verb, unit, scope }, head);
}

async function want(
  path: string,
  body: Record<string, unknown>,
  head: Record<string, string> = {},
): Promise<Record<string, unknown>> {
  io.print(`==> POST ${path}`);
  const res = await post(path, body, head);
  if (res.status !== 201) {
    throw new Error(`POST ${path} ${res.status}`);
  }
  return await res.json();
}

async function post(
  path: string,
  body: Record<string, unknown>,
  head: Record<string, string> = {},
): Promise<Response> {
  return await fetch(`${base}${path}`, {
    method: "POST",
    headers: { "content-type": "application/json", ...head },
    body: JSON.stringify(body),
  });
}

async function query(
  q: string,
  head: Record<string, string> = {},
): Promise<Record<string, unknown>> {
  const res = await fetch(`${base}/query`, {
    method: "POST",
    headers: { "content-type": "application/json", ...head },
    body: JSON.stringify({ q }),
  });
  if (!res.ok) {
    throw new Error(`query ${res.status}`);
  }
  return await res.json();
}

function bond(
  body: Record<string, unknown>,
  key: string,
): Array<Record<string, unknown>> {
  const bags = body.bags as Record<string, unknown>;
  const rows = bags?.[key];
  if (!Array.isArray(rows)) {
    throw new Error(`missing bond bag ${key}`);
  }
  return rows as Array<Record<string, unknown>>;
}

function num(value: unknown): number {
  if (typeof value !== "number") {
    throw new Error("expected number");
  }
  return value;
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

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
