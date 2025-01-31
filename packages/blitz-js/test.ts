import { create_empty_document } from "./pkg/blitz_js.js";
import { load } from "https://deno.land/x/winding@0.0.1/mod.ts";

const lib = load() as any;
const win = lib.openWindow();

console.log(lib.display, lib.screen, win.id);

const display = Deno.UnsafePointer.value(lib.display);
const screen = Deno.UnsafePointer.value(lib.screen);
const window = win.id;

const event = lib.event();
if (event) console.log(event);

console.log(display, screen, window);

await new Promise((r) => setTimeout(r, 2000));

console.log(create_empty_document(display, screen, window));
