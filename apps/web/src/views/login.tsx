import { Button, Card, Field, Note } from "@perish/react-components";
import { useState } from "react";

function back(): string {
	const raw = new URLSearchParams(globalThis.location.search).get("return");
	return raw?.startsWith("/") === true && !raw.startsWith("//") ? raw : "/";
}

export default function Login() {
	const [token, setToken] = useState("");
	const [warn, setWarn] = useState("");
	const [busy, setBusy] = useState(false);

	async function submit() {
		setBusy(true);
		setWarn("");
		try {
			const res = await fetch("/api/login", {
				method: "POST",
				headers: { "content-type": "application/json" },
				body: JSON.stringify({ token }),
			});
			if (res.ok) {
				globalThis.location.assign(back());
				return;
			}
			setWarn(
				res.status === 400 || res.status === 401
					? "That token was not accepted."
					: "Sign in failed. Try again.",
			);
		} catch {
			setWarn("Sign in failed. Try again.");
		}
		setBusy(false);
	}

	return (
		<Card title="Sign in">
			<Field label="Token" value={token} change={setToken} kind="password" />
			{warn === "" ? null : <Note text={warn} tone="warn" />}
			<Button label="Sign in" press={submit} busy={busy} wide />
		</Card>
	);
}
