// The download buttons link to /download/<system> on this site, which starts
// the newest installer (see api/download.js). This only fills in the version
// and size under each button.
const REPO = "opixdown/loudr";
const mac = document.getElementById("download-mac");
const win = document.getElementById("download-win");
const linux = document.getElementById("download-linux");
const buttons = [mac, win, linux];

// Highlight the button for the visitor's system; on a phone, neither.
const platform = (navigator.userAgentData?.platform || navigator.platform || "").toLowerCase();
const isPhone = /iphone|ipad|android/i.test(navigator.userAgent);
const own = isPhone
  ? null
  : platform.includes("win")
    ? win
    : platform.includes("mac")
      ? mac
      : platform.includes("linux")
        ? linux
        : null;
const highlight = (button) => {
  for (const other of buttons) other.classList.toggle("primary", other === button);
  // Put the highlighted system first.
  button.parentElement.prepend(button);
};
highlight(own || mac);

const megabytes = (bytes) => `${Math.round(bytes / 1_000_000)} MB`;

fetch(`https://api.github.com/repos/${REPO}/releases/latest`)
  .then((response) => (response.ok ? response.json() : Promise.reject()))
  .then((release) => {
    const version = release.tag_name;
    const dmg = release.assets.find((a) => a.name.endsWith(".dmg"));
    const msi = release.assets.find((a) => a.name.endsWith(".msi"));
    if (dmg) {
      document.getElementById("meta-mac").textContent =
        `${version} · Apple Silicon · ${megabytes(dmg.size)}`;
    }
    if (msi) {
      document.getElementById("meta-win").textContent =
        `${version} · Windows 10 and 11 · ${megabytes(msi.size)}`;
    }
    const deb = release.assets.find((a) => a.name.endsWith(".deb"));
    if (deb) {
      document.getElementById("meta-linux").textContent =
        `${version} · Ubuntu 24.04+ · ${megabytes(deb.size)}`;
    }
  })
  .catch(() => {});

fetch(`https://api.github.com/repos/${REPO}`)
  .then((response) => (response.ok ? response.json() : Promise.reject()))
  .then((repo) => {
    if (repo.stargazers_count > 0) {
      const stars = document.getElementById("stars");
      stars.textContent = `★ ${repo.stargazers_count}`;
      stars.hidden = false;
    }
  })
  .catch(() => {});

// "Buy me a coffee": paste the page's address here and the button appears.
const COFFEE_URL = "";
if (COFFEE_URL) {
  const coffee = document.getElementById("coffee");
  coffee.href = COFFEE_URL;
  coffee.hidden = false;
}

// A soft light that trails the cursor, and surfaces that light up under it.
// Skipped on touch screens and for people who ask for less motion.
const finePointer = matchMedia("(hover: hover) and (pointer: fine)").matches;
const calm = matchMedia("(prefers-reduced-motion: reduce)").matches;
if (finePointer && !calm) {
  const glow = document.querySelector(".glow");
  let x, y, targetX, targetY, frame = 0;
  const follow = () => {
    x += (targetX - x) * 0.14;
    y += (targetY - y) * 0.14;
    glow.style.transform = `translate3d(${x}px, ${y}px, 0)`;
    const settled = Math.abs(targetX - x) + Math.abs(targetY - y) < 0.3;
    frame = settled ? 0 : requestAnimationFrame(follow);
  };
  addEventListener("pointermove", (event) => {
    targetX = event.clientX;
    targetY = event.clientY;
    if (x === undefined) { x = targetX; y = targetY; }
    glow.classList.add("on");
    if (!frame) frame = requestAnimationFrame(follow);
  });
  document.documentElement.addEventListener("pointerleave", () => glow.classList.remove("on"));

  for (const surface of document.querySelectorAll("[data-spot]")) {
    surface.addEventListener("pointermove", (event) => {
      const box = surface.getBoundingClientRect();
      surface.style.setProperty("--mx", `${event.clientX - box.left}px`);
      surface.style.setProperty("--my", `${event.clientY - box.top}px`);
    });
  }
}
