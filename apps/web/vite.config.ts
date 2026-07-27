import { design } from "@jsr/perish__vite-plugin-design";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

const api =
	"^/(?:@grant|actor|asset|batch|column|comment|email|health|issue|key|label|login|milestone|mirror|note|org|package|project|pull|query|reaction|register|release|repo|review|run|runner|secret|shield|team|token|topic|variable)(?:/|$)";

export default defineConfig(({ command }) => {
	const plugins = [design(), react()];
	if (command !== "serve") {
		return { plugins };
	}
	const target = process.env.CODEHULL_API;
	const raw = process.env.SIDECAR_PORT;
	if (target === undefined || raw === undefined) {
		throw new Error("codehull web dev must be started through sidecar");
	}
	const port = Number(raw);
	if (!Number.isSafeInteger(port) || port < 1 || port > 65535) {
		throw new Error(`invalid SIDECAR_PORT: ${raw}`);
	}
	return {
		plugins,
		server: {
			host: "127.0.0.1",
			port,
			strictPort: true,
			proxy: { [api]: target },
		},
	};
});
