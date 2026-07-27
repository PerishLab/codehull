import { renderToString } from "react-dom/server";
import { expect, test } from "vitest";
import Home from "../src/views";
import Login from "../src/views/login";

test("home carries the mark", () => {
	expect(renderToString(<Home />)).toContain("codehull");
});

test("login accepts the gate token", () => {
	expect(renderToString(<Login />)).toContain("Token");
});
