import { isTauri } from "@tauri-apps/api/core";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";

/** Keep copying inside the click's user gesture, including desktop WebKit. */
export async function copyText(text: string): Promise<void> {
  if (isTauri()) {
    await writeText(text);
    return;
  }
  const previous = document.activeElement as HTMLElement | null;
  const input = document.createElement("textarea");
  input.value = text;
  input.readOnly = true;
  input.style.cssText =
    "position:fixed;left:0;top:0;width:1px;height:1px;opacity:0;pointer-events:none;";
  document.body.appendChild(input);
  input.select();
  let copied = false;
  try {
    copied = document.execCommand("copy");
  } finally {
    input.remove();
    previous?.focus({ preventScroll: true });
  }
  if (!copied) await navigator.clipboard.writeText(text);
}
