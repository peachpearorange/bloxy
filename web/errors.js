(() => {
  const MOST = 40;
  const LONGEST = 4000;
  const heard = [];
  let box = null;
  let dismissed = false;
  let drawing = false;

  const tidy = (value) =>
    value instanceof Error
      ? value.stack && value.stack.includes(value.message)
        ? value.stack
        : `${value.name}: ${value.message}\n${value.stack || ""}`
      : typeof value === "string"
        ? value
        : (() => {
            try {
              return JSON.stringify(value);
            } catch {
              return String(value);
            }
          })();

  const unstyled = (args) => {
    const [first, ...rest] = args;
    const styles = typeof first === "string" ? (first.match(/%c/g) || []).length : 0;
    return styles
      ? [first.replaceAll("%c", ""), ...rest.slice(styles)]
      : args;
  };

  const webgl = () => {
    const gl = document.createElement("canvas").getContext("webgl2");
    if (!gl) return ["WebGL2: none"];
    const named = gl.getExtension("WEBGL_debug_renderer_info");
    const vendor = gl.getParameter(named ? named.UNMASKED_VENDOR_WEBGL : gl.VENDOR);
    const renderer = gl.getParameter(named ? named.UNMASKED_RENDERER_WEBGL : gl.RENDERER);
    const software = /swiftshader|llvmpipe|software|basic render|microsoft basic/i.test(renderer);
    const limits = [
      "MAX_TEXTURE_SIZE",
      "MAX_ARRAY_TEXTURE_LAYERS",
      "MAX_TEXTURE_IMAGE_UNITS",
      "MAX_COMBINED_TEXTURE_IMAGE_UNITS",
      "MAX_UNIFORM_BLOCK_SIZE",
      "MAX_VERTEX_UNIFORM_BLOCKS",
      "MAX_FRAGMENT_UNIFORM_BLOCKS",
      "MAX_DRAW_BUFFERS",
      "MAX_SAMPLES",
    ].map((name) => `${name.slice(4).toLowerCase()} ${gl.getParameter(gl[name])}`);
    gl.getExtension("WEBGL_lose_context")?.loseContext();
    return [
      `WebGL2: ${renderer} (${vendor})${software ? "  << SOFTWARE RENDERING" : ""}`,
      `limits: ${limits.join(", ")}`,
    ];
  };

  const graphics = () => {
    const lines = [
      `page: ${location.pathname}${location.search}`,
      `browser: ${navigator.userAgent}`,
      `screen: ${innerWidth}x${innerHeight} @${devicePixelRatio}`,
      `WebGPU: ${navigator.gpu ? "present" : "none"}`,
    ];
    try {
      lines.push(...webgl());
    } catch (blame) {
      lines.push(`WebGL2: probe failed: ${blame}`);
    }
    return lines;
  };

  const button = (label, act) => {
    const pressed = document.createElement("button");
    pressed.textContent = label;
    pressed.style.cssText =
      "font: inherit; color: #ffd9d4; background: #4a1a16; border: 1px solid #c0392b; padding: 2px 10px; cursor: pointer;";
    pressed.addEventListener("click", (event) => {
      event.stopPropagation();
      act(pressed);
    });
    return pressed;
  };

  const summary = () =>
    [
      ...box.querySelector(".graphics").textContent.split("\n"),
      "",
      ...heard.map(({ text, count }, index) =>
        `${index === 0 ? "[first] " : ""}${count > 1 ? `(x${count}) ` : ""}${text}`),
    ].join("\n");

  const open = () => {
    box = document.createElement("div");
    box.id = "errors";
    box.style.cssText = [
      "position: fixed", "left: 8px", "bottom: 8px", "z-index: 10",
      "width: min(96vw, 110ch)", "max-height: 45vh", "overflow: auto",
      "background: rgba(24, 8, 8, 0.94)", "border: 2px solid #c0392b",
      "color: #ffd9d4", "font: 12px/1.35 ui-monospace, Menlo, Consolas, monospace",
      "padding: 8px", "box-sizing: border-box", "user-select: text",
    ].join(";");
    for (const kind of ["mousedown", "mouseup", "click", "wheel", "contextmenu"]) {
      box.addEventListener(kind, (event) => event.stopPropagation());
    }
    const head = document.createElement("div");
    head.style.cssText = "display: flex; gap: 8px; align-items: center; margin-bottom: 6px;";
    const title = document.createElement("strong");
    title.textContent = "Bloxy hit an error. The first one is usually the cause.";
    title.style.flex = "1";
    head.append(
      title,
      button("Copy", (pressed) =>
        navigator.clipboard?.writeText(summary()).then(
          () => (pressed.textContent = "Copied"),
          () => (pressed.textContent = "Select and copy"),
        )),
      button("Close", () => {
        dismissed = true;
        box.remove();
      }),
    );
    const details = document.createElement("pre");
    details.className = "graphics";
    details.style.cssText = "margin: 0 0 6px; white-space: pre-wrap; color: #f0c8a0;";
    details.textContent = graphics().join("\n");
    navigator.gpu?.requestAdapter().then(
      (adapter) => {
        const info = adapter?.info;
        details.textContent += `\nWebGPU adapter: ${
          adapter ? [info?.vendor, info?.architecture, info?.description].filter(Boolean).join(" ") || "yes" : "none"
        }`;
      },
      (blame) => (details.textContent += `\nWebGPU adapter: failed: ${blame}`),
    );
    const list = document.createElement("div");
    list.className = "list";
    box.append(head, details, list);
    document.body.append(box);
    if (document.fullscreenElement) document.exitFullscreen?.().catch(() => {});
    if (document.pointerLockElement) document.exitPointerLock?.();
  };

  const entry = ({ text, count }, index) => {
    const line = document.createElement("pre");
    line.style.cssText = `margin: 0; padding: 4px 0; border-top: 1px solid #5a2420; white-space: pre-wrap; word-break: break-word;${
      index === 0 ? " color: #fff; font-weight: 600;" : ""
    }`;
    line.textContent = `${index === 0 ? "[first] " : ""}${count > 1 ? `(x${count}) ` : ""}${text}`;
    return line;
  };

  const draw = () => {
    drawing = false;
    if (!document.body) addEventListener("DOMContentLoaded", draw, { once: true });
    else {
      if (!box) open();
      box.querySelector(".list").replaceChildren(...heard.map(entry));
    }
  };

  const report = (text) => {
    const said = String(text).slice(0, LONGEST);
    const known = heard.find((one) => one.text === said);
    if (known) known.count += 1;
    else if (heard.length < MOST) heard.push({ text: said, count: 1 });
    if (!drawing && !dismissed) {
      drawing = true;
      setTimeout(draw, 200);
    }
  };

  const original = console.error.bind(console);
  console.error = (...args) => {
    original(...args);
    try {
      report(unstyled(args).map(tidy).join(" "));
    } catch {}
  };

  addEventListener(
    "error",
    (event) =>
      event.target && event.target !== window
        ? report(`could not load ${event.target.src || event.target.href || event.target.tagName}`)
        : report(event.error ? tidy(event.error) : `${event.message} (${event.filename}:${event.lineno})`),
    true,
  );
  addEventListener("unhandledrejection", (event) => report(`unhandled: ${tidy(event.reason)}`));
})();
