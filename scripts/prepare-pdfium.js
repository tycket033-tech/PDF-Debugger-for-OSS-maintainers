#!/usr/bin/env node

const childProcess = require("node:child_process");
const fs = require("node:fs");
const https = require("node:https");
const net = require("node:net");
const os = require("node:os");
const path = require("node:path");
const tls = require("node:tls");

const REPO_OWNER = "bblanchon";
const REPO_NAME = "pdfium-binaries";
const RELEASE_API = `https://api.github.com/repos/${REPO_OWNER}/${REPO_NAME}/releases/latest`;
const USER_AGENT = "PDF-Debugger-for-OSS-Maintainers";
const ROOT_DIR = path.resolve(__dirname, "..");
const PDFIUM_DIR = path.join(ROOT_DIR, "src-tauri", "resources", "pdfium");
const METADATA_PATH = path.join(PDFIUM_DIR, "pdfium-runtime.json");
const FORCE = process.argv.includes("--force");
const DEFAULT_PROXY = "http://127.0.0.1:7897";

main().catch((error) => {
  console.error(`PDFium preparation failed: ${error.message}`);
  process.exitCode = 1;
});

async function main() {
  const assetName = process.env.PDFIUM_BINARY_ASSET || platformAssetName();
  const libraryName = platformLibraryName();
  const existingLibrary = path.join(PDFIUM_DIR, libraryName);

  if (!FORCE && fs.existsSync(existingLibrary)) {
    console.log(`PDFium runtime already exists: ${existingLibrary}`);
    console.log("Use `npm run pdfium:prepare -- --force` to download it again.");
    return;
  }

  console.log(`Fetching latest ${REPO_OWNER}/${REPO_NAME} release metadata...`);
  const release = await requestJson(RELEASE_API);
  const asset = release.assets?.find((candidate) => candidate.name === assetName);
  if (!asset) {
    const available = release.assets?.map((candidate) => candidate.name).join(", ") || "none";
    throw new Error(`Release ${release.tag_name || "latest"} does not contain ${assetName}. Available assets: ${available}`);
  }

  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "pdf-debugger-pdfium-"));
  const archivePath = path.join(tempDir, asset.name);
  try {
    console.log(`Downloading ${asset.name} from ${release.tag_name || "latest"}...`);
    await downloadFile(asset.browser_download_url, archivePath);

    fs.mkdirSync(PDFIUM_DIR, { recursive: true });
    cleanGeneratedRuntime(PDFIUM_DIR);
    extractArchive(archivePath, PDFIUM_DIR);
    normalizeRuntimeLayout(PDFIUM_DIR, libraryName);

    fs.writeFileSync(
      METADATA_PATH,
      JSON.stringify(
        {
          source: `${REPO_OWNER}/${REPO_NAME}`,
          release: release.tag_name || null,
          asset: asset.name,
          downloadedAt: new Date().toISOString(),
          library: libraryName,
        },
        null,
        2,
      ),
    );

    console.log(`PDFium runtime is ready: ${path.join(PDFIUM_DIR, libraryName)}`);
  } finally {
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
}

function platformAssetName() {
  const platform = process.platform;
  const arch = process.arch;

  if (platform === "win32") {
    return `pdfium-win-${mappedArch(arch)}.tgz`;
  }

  if (platform === "linux") {
    return `pdfium-linux-${mappedArch(arch)}.tgz`;
  }

  if (platform === "darwin") {
    return `pdfium-mac-${mappedArch(arch)}.tgz`;
  }

  throw new Error(`Unsupported platform for default PDFium runtime: ${platform}/${arch}`);
}

function mappedArch(arch) {
  if (arch === "x64") {
    return "x64";
  }
  if (arch === "ia32") {
    return "x86";
  }
  if (arch === "arm64") {
    return "arm64";
  }
  if (arch === "arm") {
    return "arm";
  }
  throw new Error(`Unsupported CPU architecture for PDFium runtime: ${arch}`);
}

function platformLibraryName() {
  if (process.platform === "win32") {
    return "pdfium.dll";
  }
  if (process.platform === "darwin") {
    return "libpdfium.dylib";
  }
  return "libpdfium.so";
}

function cleanGeneratedRuntime(directory) {
  for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
    if (entry.name === ".gitkeep" || entry.name === "README.md") {
      continue;
    }
    fs.rmSync(path.join(directory, entry.name), { recursive: true, force: true });
  }
}

function extractArchive(archivePath, outputDir) {
  const result = childProcess.spawnSync("tar", ["-xzf", archivePath, "-C", outputDir], {
    encoding: "utf8",
    stdio: "pipe",
  });

  if (result.status !== 0) {
    const detail = [result.stderr, result.stdout].filter(Boolean).join("\n");
    throw new Error(`Could not extract ${archivePath} with tar. ${detail}`);
  }
}

function normalizeRuntimeLayout(directory, libraryName) {
  const libraryPath = findFile(directory, libraryName);
  if (!libraryPath) {
    throw new Error(`Downloaded archive did not contain ${libraryName}`);
  }

  const rootLibraryPath = path.join(directory, libraryName);
  if (path.resolve(libraryPath) !== path.resolve(rootLibraryPath)) {
    fs.copyFileSync(libraryPath, rootLibraryPath);
  }

  if (process.platform === "win32") {
    const siblingDir = path.dirname(libraryPath);
    for (const entry of fs.readdirSync(siblingDir)) {
      if (entry.toLowerCase().endsWith(".dll")) {
        const source = path.join(siblingDir, entry);
        const target = path.join(directory, entry);
        if (path.resolve(source) !== path.resolve(target)) {
          fs.copyFileSync(source, target);
        }
      }
    }
  }
}

function findFile(directory, fileName) {
  for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
    const entryPath = path.join(directory, entry.name);
    if (entry.isFile() && entry.name === fileName) {
      return entryPath;
    }
    if (entry.isDirectory()) {
      const found = findFile(entryPath, fileName);
      if (found) {
        return found;
      }
    }
  }
  return null;
}

function requestJson(url) {
  return withNetworkFallback("GitHub release metadata request", (transport) =>
    requestText(url, {
      accept: "application/vnd.github+json",
      agent: transport.agent,
    }),
  ).then((body) => {
    try {
      return JSON.parse(body);
    } catch (error) {
      throw new Error(`Could not parse GitHub release JSON: ${error.message}`);
    }
  });
}

function requestText(url, options = {}) {
  return new Promise((resolve, reject) => {
    https
      .get(
        url,
        {
          agent: options.agent,
          headers: {
            ...(options.accept ? { Accept: options.accept } : {}),
            "User-Agent": USER_AGENT,
          },
        },
        (response) => {
          if (response.statusCode < 200 || response.statusCode >= 300) {
            response.resume();
            reject(new Error(`GitHub returned HTTP ${response.statusCode} for ${url}`));
            return;
          }

          let body = "";
          response.setEncoding("utf8");
          response.on("data", (chunk) => {
            body += chunk;
          });
          response.on("end", () => resolve(body));
        },
      )
      .on("error", reject);
  });
}

function downloadFile(url, outputPath) {
  return withNetworkFallback(`PDFium asset download from ${url}`, (transport) =>
    downloadFileWithTransport(url, outputPath, transport),
  );
}

function downloadFileWithTransport(url, outputPath, transport) {
  return new Promise((resolve, reject) => {
    const request = https.get(
      url,
      {
        agent: transport.agent,
        headers: {
          "User-Agent": USER_AGENT,
        },
      },
      (response) => {
        if (response.statusCode >= 300 && response.statusCode < 400 && response.headers.location) {
          response.resume();
          const redirectUrl = new URL(response.headers.location, url).toString();
          downloadFileWithTransport(redirectUrl, outputPath, transport).then(resolve, reject);
          return;
        }

        if (response.statusCode < 200 || response.statusCode >= 300) {
          response.resume();
          reject(new Error(`Download returned HTTP ${response.statusCode} for ${url}`));
          return;
        }

        const file = fs.createWriteStream(outputPath);
        response.pipe(file);
        file.on("finish", () => {
          file.close(resolve);
        });
        file.on("error", reject);
      },
    );
    request.on("error", reject);
  });
}

async function withNetworkFallback(description, action) {
  const transports = [
    { label: "direct", agent: undefined },
    ...proxyTransports(),
  ];
  const errors = [];

  for (let index = 0; index < transports.length; index += 1) {
    const transport = transports[index];
    try {
      if (index > 0) {
        console.log(`Retrying ${description} through ${transport.label}...`);
      }
      return await action(transport);
    } catch (error) {
      errors.push(`${transport.label}: ${error.message}`);
      if (index === 0 && transports.length > 1) {
        console.warn(`Direct ${description} failed: ${error.message}`);
      }
    }
  }

  throw new Error(`${description} failed. Attempts: ${errors.join(" | ")}`);
}

function proxyTransports() {
  const output = [];

  for (const proxy of proxyCandidates()) {
    try {
      output.push({
        label: `proxy ${proxy}`,
        agent: createHttpProxyAgent(proxy),
      });
    } catch (error) {
      output.push({
        label: `proxy ${proxy}`,
        agent: undefined,
        unsupported: error.message,
      });
    }
  }

  return output.filter((transport) => {
    if (!transport.unsupported) {
      return true;
    }
    console.warn(`Skipping ${transport.label}: ${transport.unsupported}`);
    return false;
  });
}

function proxyCandidates() {
  const configured = [
    process.env.PDFIUM_BINARY_PROXY,
    process.env.HTTPS_PROXY,
    process.env.https_proxy,
    process.env.HTTP_PROXY,
    process.env.http_proxy,
    DEFAULT_PROXY,
  ];
  const seen = new Set();
  const output = [];

  for (const value of configured) {
    const proxy = value?.trim();
    if (!proxy || seen.has(proxy)) {
      continue;
    }
    seen.add(proxy);
    output.push(proxy);
  }

  return output;
}

function createHttpProxyAgent(proxyUrl) {
  const proxy = new URL(proxyUrl);
  if (proxy.protocol !== "http:") {
    throw new Error(`Only HTTP proxies are supported for PDFium downloads: ${proxyUrl}`);
  }

  return new https.Agent({
    createConnection(options, callback) {
      let settled = false;
      const finish = (error, socket) => {
        if (settled) {
          if (socket) {
            socket.destroy();
          }
          return;
        }
        settled = true;
        callback(error, socket);
      };

      const proxyPort = Number(proxy.port || 80);
      const socket = net.connect(proxyPort, proxy.hostname);
      let response = Buffer.alloc(0);

      socket.once("connect", () => {
        const targetHost = options.host;
        const targetPort = options.port || 443;
        const authHeader =
          proxy.username || proxy.password
            ? `Proxy-Authorization: Basic ${Buffer.from(
                `${decodeURIComponent(proxy.username)}:${decodeURIComponent(proxy.password)}`,
              ).toString("base64")}\r\n`
            : "";
        socket.write(
          `CONNECT ${targetHost}:${targetPort} HTTP/1.1\r\n` +
            `Host: ${targetHost}:${targetPort}\r\n` +
            authHeader +
            "Proxy-Connection: Keep-Alive\r\n" +
            "Connection: Keep-Alive\r\n\r\n",
        );
      });

      socket.on("data", function onProxyData(chunk) {
        response = Buffer.concat([response, chunk]);
        const headerEnd = response.indexOf("\r\n\r\n");
        if (headerEnd === -1) {
          return;
        }

        socket.removeListener("data", onProxyData);
        const header = response.slice(0, headerEnd).toString("latin1");
        const status = /^HTTP\/\d(?:\.\d)?\s+(\d+)/.exec(header);
        if (!status || Number(status[1]) < 200 || Number(status[1]) >= 300) {
          socket.destroy();
          finish(new Error(`Proxy CONNECT failed through ${proxyUrl}: ${header.split("\r\n")[0] || "unknown response"}`));
          return;
        }

        const secureSocket = tls.connect(
          {
            socket,
            servername: options.servername || options.host,
          },
          () => finish(null, secureSocket),
        );
        secureSocket.once("error", (error) => finish(error));
      });

      socket.once("error", (error) => finish(error));
    },
  });
}
