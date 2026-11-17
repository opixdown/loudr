// /download/<os> on the website: starts the newest installer for that system.
//
// The installers live on GitHub Releases. This looks up the latest release
// and redirects to the right file, so the download always begins from the
// website and the visitor never lands on a GitHub page.
const REPO = "opixdown/loudr";
const RELEASES = `https://github.com/${REPO}/releases/latest`;

// The file each system gets, by how its name ends.
const EXTENSIONS = {
  windows: ".msi",
  mac: ".dmg",
  linux: ".deb",
  appimage: ".AppImage",
};

export default async function handler(request, response) {
  const extension = EXTENSIONS[String(request.query.os || "").toLowerCase()];
  if (!extension) {
    response.status(404).send("Unknown system. Try /download/windows, /download/mac or /download/linux.");
    return;
  }

  let url = RELEASES;
  try {
    const headers = { Accept: "application/vnd.github+json", "User-Agent": "loudr-site" };
    if (process.env.GITHUB_TOKEN) headers.Authorization = `Bearer ${process.env.GITHUB_TOKEN}`;
    const reply = await fetch(`https://api.github.com/repos/${REPO}/releases/latest`, { headers });
    if (reply.ok) {
      const release = await reply.json();
      const asset = release.assets.find((a) => a.name.endsWith(extension));
      if (asset) url = asset.browser_download_url;
    }
  } catch {
    // GitHub unreachable: fall back to the release page.
  }

  // Vercel's edge keeps the answer for five minutes, so GitHub is asked
  // rarely and a new release shows up soon after it is published.
  response.setHeader("Cache-Control", "public, s-maxage=300, stale-while-revalidate=3600");
  response.redirect(302, url);
}
