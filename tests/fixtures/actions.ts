import { mount } from "svelte";
import ActionScope from "./ActionScope.svelte";
import "../../src/app.css";
if (!import.meta.env.DEV) throw new Error("Browser fixtures are development-only");
mount(ActionScope, { target: document.getElementById("app")! });
