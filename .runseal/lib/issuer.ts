const ALGORITHM = { name: "ECDSA", namedCurve: "P-256" } as const;
const SIGNING = { name: "ECDSA", hash: "SHA-256" } as const;

export type Issuer = {
  url: string;
  mint: (sub: string) => Promise<string>;
  stop: () => Promise<void>;
};

function wrap(bytes: Uint8Array): string {
  let held = "";
  for (const byte of bytes) {
    held += String.fromCharCode(byte);
  }
  return btoa(held).replaceAll("+", "-").replaceAll("/", "_").replaceAll("=", "");
}

function part(value: unknown): string {
  return wrap(new TextEncoder().encode(JSON.stringify(value)));
}

export async function issuer(port: number): Promise<Issuer> {
  const pair = await crypto.subtle.generateKey(ALGORITHM, true, ["sign", "verify"]);
  const jwk = await crypto.subtle.exportKey("jwk", pair.publicKey);
  const kid = "act";
  const url = `http://127.0.0.1:${port}`;
  const keys = {
    keys: [{ kty: jwk.kty, crv: jwk.crv, x: jwk.x, y: jwk.y, kid, alg: "ES256", use: "sig" }],
  };
  const disco = { issuer: url, jwks_uri: `${url}/.well-known/jwks.json` };

  const server = Deno.serve({ port, hostname: "127.0.0.1", onListen: () => {} }, (req) => {
    const seat = new URL(req.url).pathname;
    if (seat === "/.well-known/openid-configuration") {
      return Response.json(disco);
    }
    if (seat === "/.well-known/jwks.json") {
      return Response.json(keys);
    }
    return new Response("not found", { status: 404 });
  });

  const mint = async (sub: string): Promise<string> => {
    const now = Math.floor(Date.now() / 1000);
    const head = part({ alg: "ES256", typ: "JWT", kid });
    const body = part({
      iss: url,
      sub,
      aud: url,
      kind: "access",
      scope: "openid",
      iat: now,
      exp: now + 3600,
    });
    const signed = `${head}.${body}`;
    const raw = await crypto.subtle.sign(
      SIGNING,
      pair.privateKey,
      new TextEncoder().encode(signed),
    );
    return `${signed}.${wrap(new Uint8Array(raw))}`;
  };

  server.unref();
  return { url, mint, stop: () => server.shutdown() };
}
