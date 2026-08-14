import { bin, exists } from "./cmd.ts";
import { io } from "./io.ts";
import type { Issuer } from "./issuer.ts";

const host = "127.0.0.1";
const quiet = { GIT_TERMINAL_PROMPT: "0" };

type Seat = { id: number; head: Record<string, string> };

type Source = { main: string; bundle: string };

type Held = { child: Deno.ChildProcess; drain: Promise<void>; log: () => string };

export async function haul(root: string, mint: Issuer, port: number): Promise<void> {
  const dir = `${root}/.local/haul`;
  const base = `http://${host}:${port}/api`;
  const held = await raise(root, dir, base, mint, port);
  const seed = await source(dir);
  try {
    const seat = await join(base, mint, "hauler");
    const stranger = await join(base, mint, "tourist");
    const repo = await make(base, seat);
    await want(base, `/repo/${repo}/git`, seat, "PUT");
    await want(base, `/repo/${repo}/git/object`, seat, "POST", {
      object: seed.main,
      bundle: seed.bundle,
    });
    await want(base, `/repo/${repo}/git/ref`, seat, "POST", {
      name: "refs/heads/main",
      after: seed.main,
    });
    const url = `http://${host}:${port}/api/repo/${repo}/git`;

    await check("a real git client clones what the seat holds", async () => {
      const into = `${dir}/clone`;
      const code = await clone(url, into, seat);
      if (code !== 0) {
        throw new Error(`clone exited ${code}`);
      }
      const seen = await bin("git").text(["-C", into, "rev-parse", "HEAD"]);
      if (seen !== seed.main) {
        throw new Error(`clone head ${seen} is not ${seed.main}`);
      }
      const probe = await Deno.readTextFile(`${into}/probe`);
      if (probe !== "main\n") {
        throw new Error("cloned bytes differ from the ingested bytes");
      }
    });

    await check("a real git client pushes a new commit", async () => {
      const work = `${dir}/work`;
      if (await clone(url, work, seat) !== 0) {
        throw new Error("clone for push failed");
      }
      await bin("git").text(["-C", work, "config", "user.name", "Codehull Act"]);
      await bin("git").text(["-C", work, "config", "user.email", "codehull@example.invalid"]);
      await Deno.writeTextFile(`${work}/probe`, "pushed\n");
      await bin("git").text(["-C", work, "commit", "-q", "-am", "advance main"], {
        stderr: "null",
      });
      const head = await bin("git").text(["-C", work, "rev-parse", "HEAD"]);
      if (await push(work, seat, ["origin", "main"]) !== 0) {
        throw new Error("push refused a fast-forward");
      }
      const seen = await tip(base, repo, seat, "refs/heads/main");
      if (seen !== head) {
        throw new Error(`main reads ${seen}, pushed ${head}`);
      }
      const back = `${dir}/back`;
      if (await clone(url, back, seat) !== 0) {
        throw new Error("clone after push failed");
      }
      if (await Deno.readTextFile(`${back}/probe`) !== "pushed\n") {
        throw new Error("pushed bytes did not come back");
      }
    });

    await check("a stale push is refused and writes nothing", async () => {
      const work = `${dir}/stale`;
      if (await clone(url, work, seat) !== 0) {
        throw new Error("clone for stale push failed");
      }
      const held = await tip(base, repo, seat, "refs/heads/main");
      await bin("git").text(["-C", work, "config", "user.name", "Codehull Act"]);
      await bin("git").text(["-C", work, "config", "user.email", "codehull@example.invalid"]);
      await bin("git").text(["-C", work, "reset", "-q", "--hard", "HEAD~1"], { stderr: "null" });
      await Deno.writeTextFile(`${work}/probe`, "forked\n");
      await bin("git").text(["-C", work, "commit", "-q", "-am", "fork main"], { stderr: "null" });
      if (
        await push(work, seat, [
          "--force-with-lease=refs/heads/main:" + "0".repeat(40),
          "origin",
          "main",
        ]) === 0
      ) {
        throw new Error("a stale expectation was accepted");
      }
      if (await tip(base, repo, seat, "refs/heads/main") !== held) {
        throw new Error("a refused push still moved the reference");
      }
    });

    await check("a push deletes a reference by retiring it", async () => {
      const work = `${dir}/gone`;
      if (await clone(url, work, seat) !== 0) {
        throw new Error("clone for delete failed");
      }
      if (await push(work, seat, ["origin", "HEAD:refs/heads/spare"]) !== 0) {
        throw new Error("push refused a new branch");
      }
      if (await tip(base, repo, seat, "refs/heads/spare") === null) {
        throw new Error("the new branch did not settle");
      }
      if (await push(work, seat, ["origin", "--delete", "spare"]) !== 0) {
        throw new Error("push refused a delete");
      }
      if (await tip(base, repo, seat, "refs/heads/spare") !== null) {
        throw new Error("a deleted reference still reads");
      }
    });

    await check("tags travel, plain and annotated", async () => {
      const work = `${dir}/tags`;
      if (await clone(url, work, seat) !== 0) {
        throw new Error("clone for tags failed");
      }
      const at = (args: string[]) => bin("git").text(["-C", work, ...args], { stderr: "null" });
      await at(["config", "user.name", "Codehull Act"]);
      await at(["config", "user.email", "codehull@example.invalid"]);
      await at(["tag", "plain"]);
      await at(["tag", "-a", "signed", "-m", "an annotated tag"]);
      if (await push(work, seat, ["origin", "plain", "signed"]) !== 0) {
        throw new Error("push refused tags");
      }
      const plain = await at(["rev-parse", "plain"]);
      const signed = await at(["rev-parse", "signed"]);
      if (await tip(base, repo, seat, "refs/tags/plain") !== plain) {
        throw new Error("a plain tag did not settle");
      }
      if (await tip(base, repo, seat, "refs/tags/signed") !== signed) {
        throw new Error("an annotated tag did not settle at its tag object");
      }
      const back = `${dir}/tagged`;
      if (await clone(url, back, seat) !== 0) {
        throw new Error("clone after tags failed");
      }
      const seen = await bin("git").text(["-C", back, "tag", "--list"]);
      if (!seen.includes("plain") || !seen.includes("signed")) {
        throw new Error(`clone did not bring the tags back: ${seen}`);
      }
      if (await push(work, seat, ["origin", "--delete", "plain"]) !== 0) {
        throw new Error("push refused a tag delete");
      }
      if (await tip(base, repo, seat, "refs/tags/plain") !== null) {
        throw new Error("a deleted tag still reads");
      }
    });

    await check("any namespace below refs travels", async () => {
      const work = `${dir}/notes`;
      if (await clone(url, work, seat) !== 0) {
        throw new Error("clone for namespace check failed");
      }
      if (await push(work, seat, ["origin", "HEAD:refs/notes/probe"]) !== 0) {
        throw new Error("push refused an ordinary git namespace");
      }
      const head = await bin("git").text(["-C", work, "rev-parse", "HEAD"]);
      if (await tip(base, repo, seat, "refs/notes/probe") !== head) {
        throw new Error("the reference did not settle");
      }
      const back = `${dir}/noted`;
      if (await clone(url, back, seat) !== 0) {
        throw new Error("clone after the namespace push failed");
      }
      const fetched = await bin("git").status(
        [
          "-C",
          back,
          "-c",
          `http.extraHeader=Authorization: ${seat.head.authorization}`,
          "fetch",
          "origin",
          "refs/notes/probe:refs/notes/probe",
        ],
        { env: quiet, stdin: "null", stdout: "null", stderr: "null" },
      );
      if (fetched !== 0) {
        throw new Error("fetch refused the namespace it had just accepted");
      }
    });

    await check("a name that would break the query is refused", async () => {
      const res = await send(base, `/repo/${repo}/git/ref`, seat, "POST", {
        name: 'refs/heads/a"b',
        after: seed.main,
      });
      await res.body?.cancel();
      if (res.status !== 400) {
        throw new Error(`a quoted reference name returned ${res.status}`);
      }
    });

    await check("merges join two trees for real", async () => {
      for (const mode of ["forward", "join", "squash"]) {
        const made = await send(base, "/batch", seat, "POST", {
          deeds: [{ verb: "put", unit: "Weld", fields: { mode, repo: repo } }],
        });
        await made.body?.cancel();
        if (made.status !== 200) {
          throw new Error(`permitting ${mode} returned ${made.status}`);
        }
      }
      const work = `${dir}/weld`;
      if (await clone(url, work, seat) !== 0) {
        throw new Error("clone for merge failed");
      }
      const at = (args: string[]) => bin("git").text(["-C", work, ...args], { stderr: "null" });
      await at(["config", "user.name", "Codehull Act"]);
      await at(["config", "user.email", "codehull@example.invalid"]);
      const trunk = await at(["rev-parse", "HEAD"]);

      await at(["checkout", "-q", "-b", "ahead"]);
      await Deno.writeTextFile(`${work}/ahead`, "ahead\n");
      await at(["add", "ahead"]);
      await at(["commit", "-q", "-m", "ahead"]);
      const ahead = await at(["rev-parse", "HEAD"]);
      if (await push(work, seat, ["origin", "ahead"]) !== 0) {
        throw new Error("push refused the ahead branch");
      }
      if (await merge(base, repo, seat, "forward", "ahead") !== ahead) {
        throw new Error("a forward merge did not move main to head");
      }

      await at(["checkout", "-q", "-b", "side", trunk]);
      await Deno.writeTextFile(`${work}/side`, "side\n");
      await at(["add", "side"]);
      await at(["commit", "-q", "-m", "side"]);
      if (await push(work, seat, ["origin", "side"]) !== 0) {
        throw new Error("push refused the side branch");
      }
      const joined = await merge(base, repo, seat, "join", "side");
      const back = `${dir}/joined`;
      if (await clone(url, back, seat) !== 0) {
        throw new Error("clone after join failed");
      }
      const parents = await bin("git").text(
        ["-C", back, "rev-list", "--parents", "-n", "1", joined],
      );
      if (parents.trim().split(" ").length !== 3) {
        throw new Error(`a join did not produce two parents: ${parents}`);
      }
      if (await Deno.readTextFile(`${back}/side`) !== "side\n") {
        throw new Error("the joined tree lost one side");
      }

      await at(["checkout", "-q", "-b", "flat", trunk]);
      await Deno.writeTextFile(`${work}/flat`, "flat\n");
      await at(["add", "flat"]);
      await at(["commit", "-q", "-m", "flat"]);
      if (await push(work, seat, ["origin", "flat"]) !== 0) {
        throw new Error("push refused the flat branch");
      }
      const flat = await merge(base, repo, seat, "squash", "flat");
      const seen = await bin("git").text(
        [
          "-C",
          back,
          "-c",
          `http.extraHeader=Authorization: ${seat.head.authorization}`,
          "fetch",
          "-q",
          "origin",
          "main",
        ],
        { env: quiet },
      );
      void seen;
      const single = await bin("git").text(["-C", back, "rev-list", "--parents", "-n", "1", flat]);
      if (single.trim().split(" ").length !== 2) {
        throw new Error(`a squash did not produce one parent: ${single}`);
      }
    });

    await check("an unpermitted merge is refused", async () => {
      const held = await tip(base, repo, seat, "refs/heads/main");
      const res = await send(base, `/repo/${repo}/git/merge`, seat, "POST", {
        base: "refs/heads/main",
        head: "refs/heads/side",
        mode: "rebase",
      });
      await res.body?.cancel();
      if (res.status !== 400) {
        throw new Error(`an unknown mode returned ${res.status}`);
      }
      if (await tip(base, repo, seat, "refs/heads/main") !== held) {
        throw new Error("a refused merge still moved main");
      }
    });

    await check("a commit carries verdicts by context", async () => {
      const head = await tip(base, repo, seat, "refs/heads/main");
      if (head === null) {
        throw new Error("main is absent");
      }
      const combined = async () => {
        const res = await send(
          base,
          `/repo/${repo}/git/verdict?commit=${head}`,
          seat,
          "GET",
        );
        if (res.status !== 200) {
          await res.body?.cancel();
          throw new Error(`verdict read ${res.status}`);
        }
        return await res.json() as { state: string | null; verdicts: unknown[] };
      };
      const put = async (context: string, state: string) => {
        const res = await send(base, `/repo/${repo}/git/verdict`, seat, "POST", {
          commit: head,
          context,
          state,
        });
        await res.body?.cancel();
        return res.status;
      };

      const empty = await combined();
      if (empty.state !== null || empty.verdicts.length !== 0) {
        throw new Error("a commit with no verdicts is not empty");
      }
      if (await put("guard", "pending") !== 200) {
        throw new Error("posting a verdict failed");
      }
      if ((await combined()).state !== "pending") {
        throw new Error("one pending verdict did not roll up to pending");
      }
      if (await put("guard", "success") !== 200) {
        throw new Error("replacing a verdict failed");
      }
      const once = await combined();
      if (once.state !== "success" || once.verdicts.length !== 1) {
        throw new Error(`replacing left ${once.verdicts.length} verdicts at ${once.state}`);
      }
      if (await put("ship", "failure") !== 200) {
        throw new Error("posting a second context failed");
      }
      const both = await combined();
      if (both.state !== "failure" || both.verdicts.length !== 2) {
        throw new Error(`two contexts rolled up to ${both.state}`);
      }
      if (await put("guard", "flaky") !== 400) {
        throw new Error("an unknown state was accepted");
      }
      const blind = await send(
        base,
        `/repo/${repo}/git/verdict?commit=${head}`,
        stranger,
        "GET",
      );
      await blind.body?.cancel();
      if (blind.status !== 403) {
        throw new Error(`a stranger read verdicts ${blind.status}`);
      }
    });

    await check("a stranger pushes nothing", async () => {
      const work = `${dir}/thief`;
      if (await clone(url, work, seat) !== 0) {
        throw new Error("clone for stranger push failed");
      }
      if (await push(work, stranger, ["origin", "main"]) === 0) {
        throw new Error("a stranger pushed the seat");
      }
    });

    if (await exists("ssh-keygen") && await exists("ssh")) {
      await check("a real git client clones, pushes and deletes over ssh", async () => {
        const hold = `${dir}/client`;
        await bin("ssh-keygen").text(
          ["-q", "-t", "ed25519", "-N", "", "-C", "act", "-f", hold],
          { stderr: "null" },
        );
        const print = (await Deno.readTextFile(`${hold}.pub`)).trim();
        const made = await send(base, "/batch", seat, "POST", {
          deeds: [{
            verb: "put",
            unit: "actor:key",
            fields: { title: "act", print, kind: "ssh", owner: seat.id },
          }],
        });
        await made.body?.cancel();
        if (made.status !== 200) {
          throw new Error(`registering a public key returned ${made.status}`);
        }
        const over = [
          "-c",
          `core.sshCommand=ssh -i ${hold} -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -o LogLevel=ERROR`,
        ];
        const into = `${dir}/over-ssh`;
        const code = await bin("git").status(
          [...over, "clone", "--quiet", `ssh://git@${host}:${port + 1}/hauler/haul`, into],
          { env: quiet, stdin: "null", stdout: "null", stderr: "null" },
        );
        if (code !== 0) {
          throw new Error(`ssh clone exited ${code}`);
        }
        const seen = await bin("git").text(["-C", into, "rev-parse", "HEAD"]);
        const held = await tip(base, repo, seat, "refs/heads/main");
        if (seen !== held) {
          throw new Error(`ssh clone head ${seen} is not ${held}`);
        }
        await bin("git").text(["-C", into, "config", "user.name", "Codehull Act"]);
        await bin("git").text(["-C", into, "config", "user.email", "codehull@example.invalid"]);
        await Deno.writeTextFile(`${into}/over`, "over ssh\n");
        await bin("git").text(["-C", into, "add", "over"]);
        await bin("git").text(["-C", into, "commit", "-q", "-m", "over ssh"], { stderr: "null" });
        const moved = await bin("git").text(["-C", into, "rev-parse", "HEAD"]);
        const sent = await bin("git").status([...over, "-C", into, "push", "origin", "main"], {
          env: quiet,
          stdin: "null",
          stdout: "null",
          stderr: "null",
        });
        if (sent !== 0) {
          throw new Error(`ssh push exited ${sent}`);
        }
        if (await tip(base, repo, seat, "refs/heads/main") !== moved) {
          throw new Error("an ssh push did not move main");
        }
        const gone = await bin("git").status(
          [...over, "-C", into, "push", "origin", "--delete", "flat"],
          { env: quiet, stdin: "null", stdout: "null", stderr: "null" },
        );
        if (gone !== 0) {
          throw new Error(`ssh delete exited ${gone}`);
        }
        if (await tip(base, repo, seat, "refs/heads/flat") !== null) {
          throw new Error("an ssh delete left the reference reading");
        }
        const outside = await bin("git").status(
          [...over, "-C", into, "push", "origin", "HEAD:refs/heads/main~bad"],
          { env: quiet, stdin: "null", stdout: "null", stderr: "null" },
        );
        if (outside === 0) {
          throw new Error("an invalid reference name was accepted over ssh");
        }
        const other = `${dir}/no-key`;
        const blind = await bin("git").status(
          ["clone", "--quiet", `ssh://git@${host}:${port + 1}/hauler/haul`, other],
          { env: quiet, stdin: "null", stdout: "null", stderr: "null" },
        );
        if (blind === 0) {
          throw new Error("an unregistered key cloned over ssh");
        }
      });
    } else {
      io.print("ssh absent: the ssh plane stays unproven this run");
    }

    await check("a stranger clones nothing", async () => {
      if (await clone(url, `${dir}/denied`, stranger) === 0) {
        throw new Error("a stranger cloned the seat");
      }
    });

    await check("an unsigned caller clones nothing", async () => {
      if (await clone(url, `${dir}/anon`) === 0) {
        throw new Error("an unsigned caller cloned the seat");
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

async function merge(
  base: string,
  repo: number,
  seat: Seat,
  mode: string,
  head: string,
): Promise<string> {
  const res = await send(base, `/repo/${repo}/git/merge`, seat, "POST", {
    base: "refs/heads/main",
    head: `refs/heads/${head}`,
    mode,
  });
  if (res.status !== 200) {
    await res.body?.cancel();
    throw new Error(`merge ${mode} returned ${res.status}`);
  }
  return (await res.json() as Record<string, string>).object;
}

function push(work: string, seat: Seat, args: string[]): Promise<number> {
  return bin("git").status([
    "-c",
    `http.extraHeader=Authorization: ${seat.head.authorization}`,
    "-C",
    work,
    "push",
    ...args,
  ], { env: quiet, stdin: "null", stdout: "null", stderr: "null" });
}

async function tip(
  base: string,
  repo: number,
  seat: Seat,
  name: string,
): Promise<string | null> {
  const res = await send(
    base,
    `/repo/${repo}/git/ref?name=${encodeURIComponent(name)}`,
    seat,
    "GET",
  );
  if (res.status !== 200) {
    await res.body?.cancel();
    throw new Error(`read ${name} ${res.status}`);
  }
  return (await res.json() as Record<string, string | null>).object;
}

function clone(url: string, into: string, seat?: Seat): Promise<number> {
  const args = ["clone", "--quiet", url, into];
  if (seat !== undefined) {
    args.unshift("-c", `http.extraHeader=Authorization: ${seat.head.authorization}`);
  }
  return bin("git").status(args, {
    env: quiet,
    stdin: "null",
    stdout: "null",
    stderr: "null",
  });
}

async function raise(
  root: string,
  dir: string,
  base: string,
  mint: Issuer,
  port: number,
): Promise<Held> {
  await wipe(dir);
  await Deno.mkdir(dir, { recursive: true });
  await Deno.writeTextFile(`${dir}/codehull.toml`, settings(port));
  await bin("cargo").run(["run", "--locked", "-q", "-p", "api", "--", "bootstrap", dir], {
    cwd: root,
    stdout: "null",
  });
  return await boot(root, dir, base, mint);
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
    "[ssh]",
    'key = "host.key"',
    `host = "${host}"`,
    `port = ${port + 1}`,
    "",
  ].join("\n");
}

async function boot(root: string, dir: string, base: string, mint: Issuer): Promise<Held> {
  const child = new Deno.Command("cargo", {
    args: ["run", "--locked", "-q", "-p", "api", "--", "serve", dir],
    cwd: root,
    env: { API_OIDC_ISSUER: mint.url, API_OIDC_AUDIENCE: "codehull" },
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
  const pack = `${dir}/seed.bundle`;
  await at(["bundle", "create", pack, "--all"]);
  return { main, bundle: encode(await Deno.readFile(pack)) };
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
    name: "haul",
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

async function want(
  base: string,
  path: string,
  seat: Seat,
  verb: string,
  body?: Record<string, unknown>,
): Promise<void> {
  const res = await send(base, path, seat, verb, body);
  await res.body?.cancel();
  if (res.status !== 200) {
    throw new Error(`${verb} ${path} ${res.status}`);
  }
}

function send(
  base: string,
  path: string,
  seat: Seat,
  verb: string,
  body?: Record<string, unknown>,
): Promise<Response> {
  return fetch(`${base}${path}`, {
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
