// Point the download buttons straight at the newest installers.
//
// Without JavaScript (or if GitHub can't be reached) both buttons still work:
// they lead to the latest release page, where the files are listed.
const REPO = "opixdown/loudr";
const mac = document.getElementById("download-mac");
const win = document.getElementById("download-win");

// Highlight the button for the visitor's system; on a phone, neither.
const platform = (navigator.userAgentData?.platform || navigator.platform || "").toLowerCase();
const isPhone = /iphone|ipad|android/i.test(navigator.userAgent);
if (!isPhone) {
  if (platform.includes("win")) win.classList.add("primary");
  else if (platform.includes("mac")) mac.classList.add("primary");
}
if (!mac.classList.contains("primary") && !win.classList.contains("primary")) {
  mac.classList.add("primary");
}
// Put the visitor's own system first.
if (win.classList.contains("primary")) win.parentElement.prepend(win);

const megabytes = (bytes) => `${Math.round(bytes / 1_000_000)} MB`;

fetch(`https://api.github.com/repos/${REPO}/releases/latest`)
  .then((response) => (response.ok ? response.json() : Promise.reject()))
  .then((release) => {
    const version = release.tag_name;
    const dmg = release.assets.find((a) => a.name.endsWith(".dmg"));
    const msi = release.assets.find((a) => a.name.endsWith(".msi"));
    if (dmg) {
      mac.href = dmg.browser_download_url;
      document.getElementById("meta-mac").textContent =
        `${version} · Apple Silicon · ${megabytes(dmg.size)}`;
    }
    if (msi) {
      win.href = msi.browser_download_url;
      document.getElementById("meta-win").textContent =
        `${version} · Windows 10 and 11 · ${megabytes(msi.size)}`;
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
