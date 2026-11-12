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
