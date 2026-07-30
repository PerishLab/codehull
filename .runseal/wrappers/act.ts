import { cli, flags } from "@/lib/cli.ts";
import { bin } from "@/lib/std/cmd.ts";
import { io } from "@/lib/std/io.ts";

const host = "127.0.0.1";
const port = 13400;
const origin = `http://${host}:${port}`;
const base = `${origin}/api`;

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
  `${dir}/codehull.toml`,
  `[listen]\nhost = "${host}"\nport = ${port}\nprefix = ""\n\n[store]\nkind = "memory"\n\n[cache]\nkind = "memory"\n`,
);

io.print("==> build api");
await bin("cargo").run(["build", "--locked"], { cwd: root });

io.print(`==> boot api on ${base}`);
const pg = Deno.env.get("API_STORE_URL");
const s3 = Deno.env.get("API_BLOB_ENDPOINT");
const env: Record<string, string> = pg
  ? { API_STORE_KIND: "pg", API_STORE_URL: pg, API_FRESH: "true" }
  : {};
if (s3) {
  env.API_BLOB_ENDPOINT = s3;
  env.API_BLOB_KEY = Deno.env.get("API_BLOB_KEY") ?? "codehull";
  env.API_BLOB_SECRET = Deno.env.get("API_BLOB_SECRET") ?? "codehull123";
}
if (pg) {
  io.print("==> store: postgres");
}
const child = new Deno.Command("cargo", {
  args: ["run", "--locked", "-p", "api", "--", dir],
  cwd: root,
  env,
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
  await check("the api lives only below its namespace", async () => {
    const bare = await fetch(`${origin}/health`);
    await bare.body?.cancel();
    if (bare.status !== 404) {
      throw new Error(`root health ${bare.status}`);
    }
  });
  const crown = { authorization: `sudo ${await sudo()}` };

  io.print("==> act 1: org governance");
  const ada = await join("ada");
  const bob = await join("bob");
  const cy = await join("cy");

  let org = 0;
  let crew = 0;
  await check("org creation is atomic", async () => {
    const made = await want("/org", { login: "lab" }, ada.head);
    org = num(made.id);
    const pack = await query(
      `from Team where org = "${org}" link members`,
      ada.head,
    );
    const team = roots(pack);
    if (team.length !== 1) {
      throw new Error("owners team not created atomically");
    }
    crew = num(team[0].id);
    if (bond(pack, "actor:team.members").length !== 1) {
      throw new Error("creator not enrolled atomically");
    }
    const clash = await post("/org", { login: "lab" }, bob.head);
    if (clash.status !== 403 && clash.status !== 409) {
      throw new Error(`dup org ${clash.status}`);
    }
    const after = await query(`from Actor where login = "lab" count`, ada.head);
    if (after.count !== 1) {
      throw new Error("rolled-back org left a row");
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
    const tie = bond(pack, "actor:team.members").find((t) => num(t.right) === bob.id);
    if (!tie) {
      throw new Error("bob membership tie missing");
    }
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
    const bag = roots(held);
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

  io.print("==> act 4: watch and notify");
  await check("watched public repo issues surface", async () => {
    const pub = await want("/repo", {
      name: "watched",
      visibility: "public",
      owner: ada.id,
      trunk: "main",
      archived: false,
    }, ada.head);
    const repo = num(pub.id);
    await want(`/actor/${bob.id}/watches`, { right: repo }, bob.head);
    await want("/issue", {
      title: "upstream change",
      body: "",
      closed: false,
      repo,
      author: ada.id,
    }, ada.head);
    const mine = await query(
      `from Actor where id = "${bob.id}" link watches`,
      bob.head,
    );
    const bag = (mine.bags as Record<string, unknown[]>)["actor.watches"];
    if (!Array.isArray(bag) || bag.length !== 1) {
      throw new Error("watch not recorded");
    }
    const feed = await query(
      `from Issue where repo = "${repo}" and id > "0"`,
      bob.head,
    );
    const seen = roots(feed);
    if (!Array.isArray(seen) || seen.length !== 1) {
      throw new Error(`watcher feed expected 1, got ${seen?.length}`);
    }
    const blind = await query(
      `from Issue where repo = "${repo}" and id > "0"`,
      cy.head,
    );
    void blind;
  });

  io.print("==> act 3: issue search and review");
  await check("issue search by like", async () => {
    const pub = await want("/repo", {
      name: "site",
      visibility: "public",
      owner: ada.id,
      trunk: "main",
      archived: false,
    }, ada.head);
    const repo = num(pub.id);
    await want("/issue", {
      title: "fix the login bug",
      body: "",
      closed: false,
      repo,
      author: bob.id,
    }, bob.head);
    await want("/issue", {
      title: "add dark mode",
      body: "",
      closed: false,
      repo,
      author: bob.id,
    }, bob.head);
    const hit = await query(
      'from Issue where title like "LOGIN" and closed = "false"',
      bob.head,
    );
    const rows = roots(hit);
    if (!Array.isArray(rows) || rows.length !== 1) {
      throw new Error(`search expected 1, got ${rows?.length}`);
    }
  });

  await check("pull review round-trip", async () => {
    const pub = await want("/repo", {
      name: "app",
      visibility: "public",
      owner: bob.id,
      trunk: "main",
      archived: false,
    }, bob.head);
    const repo = num(pub.id);
    const issue = num(
      (await want("/issue", {
        title: "PR: feature",
        body: "",
        closed: false,
        repo,
        author: bob.id,
      }, bob.head)).id,
    );
    const pull = num(
      (await want("/pull", {
        base: "main",
        head: "feat",
        merged: false,
        issue,
      }, bob.head)).id,
    );
    const review = num(
      (await want("/review", {
        state: "approve",
        body: "lgtm",
        pull,
        reviewer: cy.id,
      }, cy.head)).id,
    );
    await want("/note", {
      path: "src/main.rs",
      line: 10,
      body: "nit",
      review,
    }, cy.head);
    const mine = await query(
      `from Review where reviewer = "${cy.id}"`,
      cy.head,
    );
    const seen = roots(mine);
    if (!Array.isArray(seen) || seen.length < 1) {
      throw new Error("reviewer cannot see own review");
    }
    const merge = await fetch(`${base}/pull/${pull}`, {
      method: "PATCH",
      headers: { "content-type": "application/json", ...bob.head },
      body: JSON.stringify({ merged: true }),
    });
    if (!merge.ok) {
      throw new Error(`merge ${merge.status}`);
    }
    const grab = await fetch(`${base}/pull/${pull}`, {
      method: "PATCH",
      headers: { "content-type": "application/json", ...cy.head },
      body: JSON.stringify({ merged: false }),
    });
    await grab.body?.cancel();
    if (grab.status !== 403 && grab.status !== 404) {
      throw new Error(`stranger merge ${grab.status}`);
    }
  });

  io.print("==> act 6: closure delete");
  await check("repo deletion closes the subtree atomically", async () => {
    const made = await want("/repo", {
      name: "doomed",
      visibility: "private",
      owner: bob.id,
      trunk: "main",
      archived: false,
    }, bob.head);
    const repo = num(made.id);
    await want("/issue", {
      title: "orphan-to-be",
      body: "",
      closed: false,
      repo,
      author: bob.id,
    }, bob.head);
    const direct = await fetch(`${base}/repo/${repo}`, {
      method: "DELETE",
      headers: bob.head,
    });
    await direct.body?.cancel();
    if (direct.status !== 409) {
      throw new Error(`bare delete expected 409, got ${direct.status}`);
    }
    const closed = await fetch(`${base}/repo/${repo}/close`, {
      method: "POST",
      headers: bob.head,
    });
    if (closed.status !== 204) {
      throw new Error(`closure ${closed.status}`);
    }
    const gone = await fetch(`${base}/repo/${repo}`, {
      method: "DELETE",
      headers: bob.head,
    });
    await gone.body?.cancel();
    if (gone.status !== 404) {
      throw new Error(`repo lingered ${gone.status}`);
    }
  });

  io.print("==> act 8: actions metadata");
  await check("runner, run, secret, variable, deploy key", async () => {
    const made = await want("/repo", {
      name: "ci",
      visibility: "private",
      owner: bob.id,
      trunk: "main",
      archived: false,
    }, bob.head);
    const repo = num(made.id);
    await deed("repo:runner", {
      name: "linux",
      token: "rt1",
      labels: "docker,linux",
      status: "idle",
      repo,
    }, bob.head);
    await want("/run", {
      event: "push",
      status: "success",
      commit: "abc123",
      repo,
    }, bob.head);
    await deed("repo:secret", { name: "TOKEN", data: "ciphertext", repo }, bob.head);
    const dup = await one("repo:secret", { name: "TOKEN", data: "other", repo }, bob.head);
    if (dup.status !== 409) {
      throw new Error(`secret name not scoped-unique ${dup.status}`);
    }
    await want("/variable", { name: "REGION", value: "eu", repo }, bob.head);
    await deed("repo:key", { title: "deploy", print: "ssh-ed25519 AAAA", repo }, bob.head);

    const owned = await query(`from repo:secret where repo = "${repo}"`, bob.head);
    const kept = roots(owned);
    if (!Array.isArray(kept) || kept.length !== 1) {
      throw new Error("owner cannot see own secret");
    }
    const blind = await query(`from repo:secret where repo = "${repo}"`, cy.head);
    const seen = roots(blind);
    if (Array.isArray(seen) && seen.length !== 0) {
      throw new Error("stranger sees repo secret");
    }
  });

  io.print("==> act 9: planning and delivery");
  await check("projects, releases, branch protection, packages", async () => {
    const made = await want("/repo", {
      name: "product",
      visibility: "private",
      owner: bob.id,
      trunk: "main",
      archived: false,
    }, bob.head);
    const repo = num(made.id);
    const project = num(
      (await want("/project", {
        title: "roadmap",
        closed: false,
        repo,
      }, bob.head)).id,
    );
    const column = num(
      (await want("/column", {
        title: "todo",
        sort: 1,
        project,
      }, bob.head)).id,
    );
    const issue = num(
      (await want("/issue", {
        title: "card",
        body: "",
        closed: false,
        repo,
        author: bob.id,
      }, bob.head)).id,
    );
    await want(`/column/${column}/cards`, { right: issue, spot: 3 }, bob.head);

    await want("/release", {
      tag: "v1.0",
      title: "one",
      body: "notes",
      draft: false,
      repo,
      author: bob.id,
    }, bob.head);
    const dup = await post("/release", {
      tag: "v1.0",
      title: "dup",
      body: "",
      draft: true,
      repo,
      author: bob.id,
    }, bob.head);
    if (dup.status !== 409) {
      throw new Error(`release tag not scoped-unique ${dup.status}`);
    }

    await want("/shield", {
      branch: "main",
      force: false,
      approvals: 2,
      repo,
    }, bob.head);
    await want("/package", {
      name: "libcore",
      kind: "cargo",
      version: "0.1.0",
      repo,
    }, bob.head);

    const owned = await query(`from Release where repo = "${repo}"`, bob.head);
    if (roots(owned).length !== 1) {
      throw new Error("owner cannot see own release");
    }
    const blind = await query(`from Package where repo = "${repo}"`, cy.head);
    const seen = roots(blind);
    if (Array.isArray(seen) && seen.length !== 0) {
      throw new Error("stranger sees private package");
    }
  });

  io.print("==> act 10: trimmings");
  await check("fork, archive, milestone, assignees, blocks, stars, email", async () => {
    const up = num(
      (await want("/repo", {
        name: "upstream",
        visibility: "public",
        owner: ada.id,
        trunk: "main",
        archived: false,
      }, ada.head)).id,
    );
    const fork = num(
      (await want("/repo", {
        name: "fork",
        visibility: "public",
        owner: bob.id,
        trunk: "main",
        archived: false,
        fork: up,
      }, bob.head)).id,
    );
    const forks = await query(`from Repo where fork = "${up}"`, bob.head);
    if (roots(forks).length !== 1) {
      throw new Error("fork not linked");
    }
    const arch = await fetch(`${base}/repo/${fork}`, {
      method: "PATCH",
      headers: { "content-type": "application/json", ...bob.head },
      body: JSON.stringify({ archived: true }),
    });
    if (!arch.ok) {
      throw new Error(`archive ${arch.status}`);
    }

    const mile = num(
      (await want("/milestone", {
        title: "v2",
        due: 0,
        closed: false,
        repo: fork,
      }, bob.head)).id,
    );
    const issue = num(
      (await want("/issue", {
        title: "planned",
        body: "",
        closed: false,
        repo: fork,
        author: bob.id,
        milestone: mile,
      }, bob.head)).id,
    );
    await want(`/issue/${issue}/assignees`, { right: bob.id }, bob.head);
    const other = num(
      (await want("/issue", {
        title: "blocker",
        body: "",
        closed: false,
        repo: fork,
        author: bob.id,
      }, bob.head)).id,
    );
    await want(`/issue/${issue}/blocks`, { right: other }, bob.head);
    const pack = await query(
      `from Issue where id = "${issue}" link assignees link blocks`,
      bob.head,
    );
    const bags = pack.bags as Record<string, unknown[]>;
    if (bags["repo:issue.assignees"].length !== 1 || bags["repo:issue.blocks"].length !== 1) {
      throw new Error("assignee or block missing");
    }

    await want(`/actor/${bob.id}/stars`, { right: up }, bob.head);
    const stars = await query(`from Actor where stars has "${up}" count`, bob.head);
    if (stars.count !== 1) {
      throw new Error("star not counted");
    }
    await want("/email", {
      mail: "bob@lab.dev",
      primary: true,
      actor: bob.id,
    }, bob.head);
  });

  io.print("==> act 11: org-scoped labels, runners, secrets");
  await check("org owner manages org-scoped entities via team subtree", async () => {
    const org = num((await want("/org", { login: "guild" }, ada.head)).id);
    await deed("actor:label", { name: "triage", color: "ff0", org }, ada.head);
    const dup = await one("actor:label", { name: "triage", color: "0ff", org }, ada.head);
    if (dup.status !== 409) {
      throw new Error(`org label name not scoped-unique ${dup.status}`);
    }
    await deed("actor:runner", {
      name: "shared",
      token: "ort1",
      labels: "linux",
      status: "idle",
      org,
    }, ada.head);
    await deed("actor:secret", { name: "DEPLOY", data: "cipher", org }, ada.head);

    const owned = await query(`from actor:secret where org = "${org}"`, ada.head);
    if (roots(owned).length !== 1) {
      throw new Error("owner cannot see org secret");
    }
    const blind = await query(`from actor:secret where org = "${org}"`, cy.head);
    const seen = roots(blind);
    if (Array.isArray(seen) && seen.length !== 0) {
      throw new Error("non-member sees org secret");
    }
    const steal = await one("actor:secret", { name: "GRAB", data: "x", org }, cy.head);
    if (steal.status !== 403) {
      throw new Error(`non-member wrote org secret ${steal.status}`);
    }
  });

  io.print("==> act 7: suspension");
  await check("suspended operator is refused", async () => {
    const made = await want(
      "/register",
      { login: "banned", kind: "user", barred: false },
      {},
    );
    const id = num(made.id);
    const token = made.token as string;
    const head = { authorization: `token ${token}` };
    const vault = num(
      (await want("/repo", {
        name: "secret",
        visibility: "private",
        owner: id,
        trunk: "main",
        archived: false,
      }, head)).id,
    );
    const before = await fetch(`${base}/repo/${vault}`, { headers: head });
    await before.body?.cancel();
    if (before.status !== 200) {
      throw new Error(`operator cannot see own repo ${before.status}`);
    }
    const bar = await fetch(`${base}/actor/${id}`, {
      method: "PATCH",
      headers: { "content-type": "application/json", ...crown },
      body: JSON.stringify({ barred: true }),
    });
    await bar.body?.cancel();
    if (!bar.ok) {
      throw new Error(`suspend ${bar.status}`);
    }
    const after = await fetch(`${base}/repo/${vault}`, { headers: head });
    await after.body?.cancel();
    if (after.status !== 404) {
      throw new Error(`suspended operator still resolves ${after.status}`);
    }
  });

  if (!s3) {
    io.print("API_BLOB_ENDPOINT absent: the blob plane stays unproven this run");
  }
  if (s3) {
    io.print("==> act 5: blobs");
    await bucket(s3);
    await check("presigned upload, gated download", async () => {
      const made = await want(
        "/asset",
        { name: "logo.png", mime: "image/png", size: 5, hash: "h1", owner: bob.id },
        bob.head,
      );
      const put = made.put as string;
      const body = new Uint8Array([1, 2, 3, 4, 5]);
      const up = await fetch(put, { method: "PUT", body });
      if (!up.ok) {
        throw new Error(`presigned put ${up.status}`);
      }
      const id = num(made.id);
      const seen = await fetch(`${base}/asset/${id}`, {
        headers: bob.head,
        redirect: "manual",
      });
      if (seen.status !== 302) {
        throw new Error(`expected 302, got ${seen.status}`);
      }
      const where = seen.headers.get("location") ?? "";
      await seen.body?.cancel();
      const got = await fetch(where);
      const back = new Uint8Array(await got.arrayBuffer());
      if (back.length !== 5 || back[0] !== 1 || back[4] !== 5) {
        throw new Error("bytes round-trip mismatch");
      }
      const blind = await fetch(`${base}/asset/${id}`, {
        headers: cy.head,
        redirect: "manual",
      });
      await blind.body?.cancel();
      if (blind.status !== 404) {
        throw new Error(`stranger expected 404, got ${blind.status}`);
      }
    });
  }

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
  const made = await want("/register", { login, kind: "user", barred: false });
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

function roots(body: Record<string, unknown>): Array<Record<string, unknown>> {
  const bags = body.bags as Record<string, unknown> | undefined;
  const rows = bags?.[body.root as string];
  return Array.isArray(rows) ? (rows as Array<Record<string, unknown>>) : [];
}

async function one(
  unit: string,
  fields: Record<string, unknown>,
  head: Record<string, string> = {},
): Promise<Response> {
  return await post("/batch", { deeds: [{ verb: "put", unit, fields }] }, head);
}

async function deed(
  unit: string,
  fields: Record<string, unknown>,
  head: Record<string, string> = {},
): Promise<number> {
  const res = await one(unit, fields, head);
  if (res.status !== 200) {
    throw new Error(`batch put ${unit} ${res.status}`);
  }
  const ids = (await res.json()).ids as number[];
  return ids[0];
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

async function sudo(): Promise<string> {
  for (let i = 0; i < 40; i++) {
    const hit = boot.match(/sudo token ([0-9a-f]+)/);
    if (hit) {
      return hit[1];
    }
    await sleep(250);
  }
  throw new Error("no sudo token in boot log");
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

async function bucket(_endpoint: string): Promise<void> {
  const make = new Deno.Command("docker", {
    args: [
      "compose",
      "exec",
      "-T",
      "minio",
      "mc",
      "mb",
      "-p",
      "local/codehull",
    ],
    stdout: "null",
    stderr: "null",
  });
  const set = new Deno.Command("docker", {
    args: [
      "compose",
      "exec",
      "-T",
      "minio",
      "mc",
      "alias",
      "set",
      "local",
      "http://127.0.0.1:9000",
      "codehull",
      "codehull123",
    ],
    stdout: "null",
    stderr: "null",
  });
  await set.output();
  await make.output();
}
